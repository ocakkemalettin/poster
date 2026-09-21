use crate::commands::{Auth, LoadTestConfig, VarOverride};
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

type LoadTestMap = Arc<Mutex<HashMap<String, LoadTestHandle>>>;

#[derive(Debug, Clone, Serialize)]
pub struct LoadTestStats {
    pub completed: u64,
    pub total: u64,
    pub successes: u64,
    pub failures: u64,
    pub avg_ms: f64,
    pub p95_ms: f64,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoadTestReport {
    pub total: u64,
    pub successes: u64,
    pub failures: u64,
    pub duration_ms: u64,
    pub avg_ms: f64,
    pub p95_ms: f64,
    pub rps_actual: f64,
    pub per_request: Vec<PerRequest>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PerRequest {
    pub seq: u64,
    pub status: u16,
    pub latency_ms: u64,
    pub success: bool,
}

pub struct LoadTestHandle {
    pub cancel: Arc<AtomicBool>,
    pub start_time: std::time::Instant,
    pub total: u64,
    pub completed: Arc<AtomicU64>,
    pub successes: Arc<AtomicU64>,
    pub failures: Arc<AtomicU64>,
}

impl LoadTestHandle {
    pub fn stats(&self) -> LoadTestStats {
        LoadTestStats {
            completed: self.completed.load(Ordering::Relaxed),
            total: self.total,
            successes: self.successes.load(Ordering::Relaxed),
            failures: self.failures.load(Ordering::Relaxed),
            avg_ms: 0.0,
            p95_ms: 0.0,
            elapsed_ms: self.start_time.elapsed().as_millis() as u64,
        }
    }
}

struct LatencyTracker {
    window: VecDeque<u64>,
    total: u64,
    sum: u128,
}

impl LatencyTracker {
    fn new() -> Self {
        Self {
            window: VecDeque::new(),
            total: 0,
            sum: 0,
        }
    }

    fn push(&mut self, ms: u64) {
        if self.window.len() >= 50_000 {
            let old = self.window.pop_front().unwrap_or(0);
            self.sum = self.sum.saturating_sub(old as u128);
        }
        self.window.push_back(ms);
        self.total += 1;
        self.sum += ms as u128;
    }

    fn snapshot(&self) -> (f64, f64) {
        if self.total == 0 {
            return (0.0, 0.0);
        }
        let avg = self.sum as f64 / self.total as f64;
        let mut v: Vec<u64> = self.window.iter().copied().collect();
        v.sort_unstable();
        let idx = ((v.len() as f64) * 0.95).min(v.len() as f64 - 1.0) as usize;
        (avg, v[idx] as f64)
    }
}

fn pick_value(o: &VarOverride) -> String {
    match o.mode.as_str() {
        "range" => {
            let start: i128 = o.value.parse().unwrap_or(0);
            let end: i128 = o
                .value_end
                .as_deref()
                .and_then(|e| e.parse().ok())
                .unwrap_or(start);
            if start >= end {
                return start.to_string();
            }
            (start + fastrand::u64(0..(end - start + 1) as u64) as i128).to_string()
        }
        "list" => {
            let items: Vec<String> = o
                .value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if items.is_empty() {
                return String::new();
            }
            items[fastrand::usize(0..items.len())].clone()
        }
        _ => o.value.clone(),
    }
}

pub fn resolve_template(tpl: &str, vars: &[(String, String)], overrides: &[VarOverride]) -> String {
    let mut out = tpl.to_string();
    for o in overrides {
        if o.key.is_empty() {
            continue;
        }
        let placeholder = format!("{{{{{}}}}}", o.key);
        if out.contains(&placeholder) {
            out = out.replace(&placeholder, &pick_value(o));
        }
    }
    for (k, v) in vars {
        out = out.replace(&format!("{{{{{}}}}}", k), v);
    }
    out
}

async fn run_one_request(
    client: &reqwest::Client,
    url_tpl: &str,
    method: &str,
    headers: &[(String, String)],
    body_type: &str,
    body_tpl: &str,
    auth: &Auth,
    vars: &[(String, String)],
    overrides: &[VarOverride],
) -> Result<(reqwest::StatusCode, String), String> {
    let url = resolve_template(url_tpl, vars, overrides);
    if url.is_empty() {
        return Err("Empty URL".into());
    }
    let mut req = match method.as_ref() {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "PATCH" => client.patch(&url),
        "DELETE" => client.delete(&url),
        "HEAD" => client.head(&url),
        other => return Err(format!("Unsupported method: {}", other)),
    };

    for (k, v) in headers {
        req = req.header(k.as_str(), resolve_template(v, vars, overrides));
    }

    use crate::commands::AuthType;
    match auth.type_ {
        AuthType::Bearer => {
            let token = resolve_template(&auth.token, vars, overrides);
            if !token.is_empty() {
                req = req.header("Authorization", format!("Bearer {}", token));
            }
        }
        AuthType::Basic => {
            let user = resolve_template(&auth.username, vars, overrides);
            let pass = resolve_template(&auth.password, vars, overrides);
            let creds = base64::engine::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                format!("{}:{}", user, pass),
            );
            req = req.header("Authorization", format!("Basic {}", creds));
        }
        AuthType::ApiKey => {
            let key = resolve_template(&auth.key, vars, overrides);
            let value = resolve_template(&auth.value, vars, overrides);
            if !key.is_empty() {
                if auth.in_header {
                    req = req.header(key.as_str(), value);
                } else {
                    req = req.query(&[(key, value)]);
                }
            }
        }
        AuthType::None => {}
    }

    if !body_tpl.is_empty() {
        let body = resolve_template(body_tpl, vars, overrides);
        match body_type.as_ref() {
            "json" => req = req.body(body),
            "form" => {
                let pairs: Vec<(String, String)> = body
                    .lines()
                    .filter_map(|line| line.split_once('='))
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();
                req = req.form(&pairs);
            }
            _ => {
                req = req.header("Content-Type", "text/plain").body(body);
            }
        }
    }

    let resp = req.send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    Ok((status, text))
}

/// Must be called from within a Tokio runtime (Tauri runs async commands there).
pub async fn spawn_load_test(
    app: AppHandle,
    map: LoadTestMap,
    id: String,
    cfg: LoadTestConfig,
) -> Result<(), String> {
    let cancel = Arc::new(AtomicBool::new(false));
    let completed = Arc::new(AtomicU64::new(0));
    let successes = Arc::new(AtomicU64::new(0));
    let failures = Arc::new(AtomicU64::new(0));
    let start_time = std::time::Instant::now();

    let rps: u64 = cfg.rps.max(1);
    let total = cfg.total_requests;
    let workers = cfg.concurrency.clamp(1, 256) as usize;
    let timeout_ms = cfg.timeout_ms.max(100);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .build()
        .map_err(|e| e.to_string())?;

    // Register the handle BEFORE spawning any worker so a fast run can never
    // finish (and be cleaned up) before it is tracked.
    let handle = LoadTestHandle {
        cancel: cancel.clone(),
        start_time,
        total,
        completed: completed.clone(),
        successes: successes.clone(),
        failures: failures.clone(),
    };
    map.lock().unwrap().insert(id.clone(), handle);

    let url_tpl = cfg.url.clone();
    let method = cfg.method.clone().to_uppercase();
    let headers: Vec<(String, String)> = cfg
        .headers
        .iter()
        .filter(|h| !h.key.is_empty())
        .map(|h| (h.key.clone(), h.value.clone()))
        .collect();
    let body_type = cfg.body_type.clone();
    let body_tpl = cfg.body.clone();
    let auth = cfg.auth.clone();
    let vars: Vec<(String, String)> = cfg.variables.iter().cloned().collect();
    let overrides: Vec<VarOverride> = cfg.overrides.clone();

    let tracker = Arc::new(std::sync::Mutex::new(LatencyTracker::new()));

    // One tokio mpsc channel per worker so each worker owns a receiver.
    let mut job_txs: Vec<tokio::sync::mpsc::UnboundedSender<u64>> = Vec::with_capacity(workers);
    let mut job_rxs: Vec<tokio::sync::mpsc::UnboundedReceiver<u64>> = Vec::with_capacity(workers);
    for _ in 0..workers {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        job_txs.push(tx);
        job_rxs.push(rx);
    }

    // Result channel: workers -> collector
    let (res_tx, mut res_rx) = tokio::sync::mpsc::unbounded_channel::<(u64, u16, u64, bool)>();

    // Pacer: token bucket at `rps`, round-robin dispatch to workers
    {
        let cancel2 = cancel.clone();
        let txs = job_txs.clone();
        tokio::spawn(async move {
            let interval = std::time::Duration::from_secs_f64(1.0 / rps as f64);
            let mut next_tick = std::time::Instant::now();
            for seq in 0..total {
                if cancel2.load(Ordering::Relaxed) {
                    break;
                }
                let w = (seq as usize) % txs.len();
                if txs[w].send(seq).is_err() {
                    break;
                }
                next_tick += interval;
                let now = std::time::Instant::now();
                if now < next_tick {
                    tokio::time::sleep(next_tick - now).await;
                } else {
                    next_tick = now;
                }
            }
        });
    }

    // Workers: pull jobs, execute requests
    for rx in job_rxs.into_iter() {
        let client = client.clone();
        let url_tpl = url_tpl.clone();
        let method = method.clone();
        let headers = headers.clone();
        let body_type = body_type.clone();
        let body_tpl = body_tpl.clone();
        let auth = auth.clone();
        let vars = vars.clone();
        let overrides = overrides.clone();
        let completed = completed.clone();
        let successes = successes.clone();
        let failures = failures.clone();
        let tracker = tracker.clone();
        let res_tx = res_tx.clone();

        tokio::spawn(async move {
            let mut rx = rx;
            while let Some(seq) = rx.recv().await {
                let t0 = std::time::Instant::now();
                let result = run_one_request(
                    &client,
                    &url_tpl,
                    &method,
                    &headers,
                    &body_type,
                    &body_tpl,
                    &auth,
                    &vars,
                    &overrides,
                )
                .await;
                let ms = t0.elapsed().as_millis() as u64;
                let (status, ok) = match result {
                    Ok((s, _)) => (s.as_u16(), s.is_success()),
                    Err(_) => (0, false),
                };
                if ok {
                    successes.fetch_add(1, Ordering::Relaxed);
                } else {
                    failures.fetch_add(1, Ordering::Relaxed);
                }
                completed.fetch_add(1, Ordering::Relaxed);
                tracker.lock().unwrap().push(ms);
                let _ = res_tx.send((seq, status, ms, ok));
            }
        });
    }

    // Drop the pacer's senders so workers see channel closure after all jobs are dispatched.
    drop(job_txs);
    drop(res_tx);

    // Stats emitter + final report collector
    {
        let app2 = app.clone();
        let completed2 = completed.clone();
        let successes2 = successes.clone();
        let failures2 = failures.clone();
        let tracker2 = tracker.clone();
        let cancel2 = cancel.clone();
        let start2 = start_time;

        tokio::spawn(async move {
            let mut results: Vec<(u64, u16, u64, bool)> = Vec::new();
            loop {
                tokio::select! {
                    maybe = res_rx.recv() => {
                        match maybe {
                            Some(r) => results.push(r),
                            None => break, // all senders dropped: done
                        }
                    }
                    _ = tokio::time::sleep(if cancel2.load(Ordering::Relaxed) {
                        std::time::Duration::from_millis(50)
                    } else {
                        std::time::Duration::from_millis(500)
                    }) => {
                        let done = completed2.load(Ordering::Relaxed);
                        let (avg, p95) = tracker2.lock().unwrap().snapshot();
                        let stats = LoadTestStats {
                            completed: done,
                            total,
                            successes: successes2.load(Ordering::Relaxed),
                            failures: failures2.load(Ordering::Relaxed),
                            avg_ms: avg,
                            p95_ms: p95,
                            elapsed_ms: start2.elapsed().as_millis() as u64,
                        };
                        let _ = app2.emit("load-test-stats", &stats);
                        if done >= total || cancel2.load(Ordering::Relaxed) {
                            break;
                        }
                    }
                }
            }

            // Drain any remaining results
            loop {
                match res_rx.try_recv() {
                    Ok(r) => results.push(r),
                    Err(_) => break,
                }
            }

            // Cap the detail payload so the IPC message stays small even for huge runs.
            const MAX_DETAIL: usize = 10_000;
            let mut results: Vec<(u64, u16, u64, bool)> = results;
            if results.len() > MAX_DETAIL {
                // keep first + last samples so the report still shows start/end behavior
                let head = &results[..MAX_DETAIL / 2];
                let tail_start = results.len() - MAX_DETAIL / 2;
                let mut kept: Vec<(u64, u16, u64, bool)> = head.to_vec();
                kept.extend_from_slice(&results[tail_start..]);
                results = kept;
            }

            let mut per_request: Vec<PerRequest> = results
                .into_iter()
                .map(|(seq, status, ms, success)| PerRequest {
                    seq,
                    status,
                    latency_ms: ms,
                    success,
                })
                .collect();
            per_request.sort_by_key(|p| p.seq);

            let (avg, p95) = tracker2.lock().unwrap().snapshot();
            let duration_ms = start2.elapsed().as_millis() as u64;
            let done = completed2.load(Ordering::Relaxed);
            let report = LoadTestReport {
                total: done,
                successes: successes2.load(Ordering::Relaxed),
                failures: failures2.load(Ordering::Relaxed),
                duration_ms,
                avg_ms: avg,
                p95_ms: p95,
                rps_actual: if duration_ms > 0 {
                    done as f64 / (duration_ms as f64 / 1000.0)
                } else {
                    0.0
                },
                per_request,
            };
            let _ = app2.emit("load-test-finished", &report);
        });
    }

    Ok(())
}
