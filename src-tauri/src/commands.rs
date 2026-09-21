use crate::db::{self, Db, Extract, Project, Request, Variable};
use crate::load_test::{LoadTestHandle, LoadTestStats, spawn_load_test};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

pub struct AppState {
    pub db: Db,
    pub load_tests: Arc<std::sync::Mutex<std::collections::HashMap<String, LoadTestHandle>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Header {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    #[default]
    None,
    Bearer,
    Basic,
    ApiKey,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Auth {
    #[serde(default)]
    pub type_: AuthType,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub in_header: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarOverride {
    pub key: String,
    /// "fixed" | "range" | "list"
    pub mode: String,
    /// fixed value; range start; comma-separated list
    pub value: String,
    /// range end (only for mode = "range")
    #[serde(default)]
    pub value_end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestConfig {
    pub url: String,
    pub method: String,
    pub headers: Vec<Header>,
    pub body_type: String,
    pub body: String,
    pub auth: Auth,
    pub variables: Vec<(String, String)>,
    pub overrides: Vec<VarOverride>,
    pub rps: u64,
    pub total_requests: u64,
    pub concurrency: u64,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseData {
    pub status: u16,
    pub status_text: String,
    pub time_ms: u64,
    pub size_bytes: u64,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CurlImportResult {
    pub method: String,
    pub url: String,
    pub headers: Vec<Header>,
    pub body_type: String,
    pub body: String,
    pub auth: Auth,
    pub warnings: Vec<String>,
}

fn vars_map(vars: &[Variable]) -> HashMap<String, String> {
    vars.iter()
        .map(|v| (v.key.clone(), v.value.clone()))
        .collect()
}

fn resolve(tpl: &str, vars: &HashMap<String, String>) -> String {
    let mut out = tpl.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("{{{{{}}}}}", k), v);
    }
    out
}

fn build_request(
    client: &reqwest::Client,
    method: &str,
    url: &str,
    headers: &[Header],
    body_type: &str,
    body: &str,
    auth: &Auth,
    vars: &HashMap<String, String>,
) -> Result<reqwest::RequestBuilder, String> {
    let url = resolve(url, vars);
    if url.is_empty() {
        return Err("URL is empty".into());
    }
    let mut req = match method.to_uppercase().as_str() {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "PATCH" => client.patch(&url),
        "DELETE" => client.delete(&url),
        "HEAD" => client.head(&url),
        other => return Err(format!("Unsupported method: {}", other)),
    };

    for h in headers {
        if h.key.is_empty() {
            continue;
        }
        req = req.header(h.key.as_str(), resolve(&h.value, vars));
    }

    match auth.type_ {
        AuthType::Bearer => {
            let token = resolve(&auth.token, vars);
            if !token.is_empty() {
                req = req.header("Authorization", format!("Bearer {}", token));
            }
        }
        AuthType::Basic => {
            let user = resolve(&auth.username, vars);
            let pass = resolve(&auth.password, vars);
            let creds = base64::engine::general_purpose::STANDARD
                .encode(format!("{}:{}", user, pass));
            req = req.header("Authorization", format!("Basic {}", creds));
        }
        AuthType::ApiKey => {
            let key = resolve(&auth.key, vars);
            let value = resolve(&auth.value, vars);
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

    if !body.is_empty() {
        let body = resolve(body, vars);
        match body_type {
            "json" => {
                req = req.header("Content-Type", "application/json");
                req = req.body(body);
            }
            "form" => {
                let pairs: Vec<(String, String)> = body
                    .lines()
                    .filter_map(|line| line.split_once('='))
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();
                req = req.form(&pairs);
            }
            "form-data" => {
                let pairs: Vec<(String, String)> = body
                    .lines()
                    .filter_map(|line| line.split_once('='))
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();
                let mut form = reqwest::multipart::Form::new();
                for (k, v) in pairs {
                    form = form.text(k, v);
                }
                req = req.multipart(form);
            }
            _ => {
                req = req.header("Content-Type", "text/plain");
                req = req.body(body);
            }
        }
    }

    Ok(req)
}

fn parse_headers_json(s: &str) -> Vec<Header> {
    serde_json::from_str(s).unwrap_or_default()
}

fn parse_auth_json(s: &str) -> Auth {
    serde_json::from_str(s).unwrap_or_default()
}

// ---------- Project commands ----------

#[tauri::command]
pub fn list_projects(state: State<AppState>) -> Result<Vec<Project>, String> {
    state.db.list_projects().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_project(state: State<AppState>, name: String) -> Result<Project, String> {
    state
        .db
        .create_project(&name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_project(
    state: State<AppState>,
    project_id: i64,
    name: String,
) -> Result<(), String> {
    state
        .db
        .rename_project(project_id, &name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_project(state: State<AppState>, project_id: i64) -> Result<(), String> {
    state
        .db
        .delete_project(project_id)
        .map_err(|e| e.to_string())
}

// ---------- Request commands ----------

#[tauri::command]
pub fn list_requests(state: State<AppState>, project_id: i64) -> Result<Vec<Request>, String> {
    state
        .db
        .list_requests(project_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_request(
    state: State<AppState>,
    project_id: i64,
    name: String,
) -> Result<Request, String> {
    state
        .db
        .create_request(project_id, &name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_request(state: State<AppState>, request: Request) -> Result<(), String> {
    state
        .db
        .update_request(&request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_request(state: State<AppState>, request_id: i64) -> Result<(), String> {
    state
        .db
        .delete_request(request_id)
        .map_err(|e| e.to_string())
}

// ---------- Variable commands ----------

#[tauri::command]
pub fn list_variables(state: State<AppState>, project_id: i64) -> Result<Vec<Variable>, String> {
    state
        .db
        .list_variables(project_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn upsert_variable(
    state: State<AppState>,
    project_id: i64,
    key: String,
    value: String,
) -> Result<(), String> {
    state
        .db
        .upsert_variable(project_id, &key, &value)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_variable(state: State<AppState>, variable_id: i64) -> Result<(), String> {
    state
        .db
        .delete_variable(variable_id)
        .map_err(|e| e.to_string())
}

// ---------- Extract commands ----------

#[tauri::command]
pub fn list_extracts(state: State<AppState>, request_id: i64) -> Result<Vec<Extract>, String> {
    state
        .db
        .list_extracts(request_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_extract(state: State<AppState>, extract: Extract) -> Result<Extract, String> {
    state
        .db
        .create_extract(&extract)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_extract(state: State<AppState>, extract_id: i64) -> Result<(), String> {
    state
        .db
        .delete_extract(extract_id)
        .map_err(|e| e.to_string())
}

// ---------- Send request + extract ----------

#[tauri::command]
pub async fn send_request(
    app: AppHandle,
    state: State<'_, AppState>,
    request_id: i64,
) -> Result<ResponseData, String> {
    let req = state
        .db
        .get_request(request_id)
        .map_err(|e| e.to_string())?
        .ok_or("Request not found".to_string())?;
    let project_vars = state
        .db
        .list_variables(req.project_id)
        .map_err(|e| e.to_string())?;
    let vars = vars_map(&project_vars);

    let headers = parse_headers_json(&req.headers_json);
    let auth = parse_auth_json(&req.auth_json);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let builder = build_request(
        &client,
        &req.method,
        &req.url,
        &headers,
        &req.body_type,
        &req.body,
        &auth,
        &vars,
    )?;

    let t0 = std::time::Instant::now();
    let resp = builder.send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let resp_headers: Vec<(String, String)> = resp
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let body = resp.text().await.map_err(|e| e.to_string())?;
    let time_ms = t0.elapsed().as_millis() as u64;

    // Apply extracts: pull JSONPath values from this response into project variables
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
        if let Ok(extracts) = state.db.list_extracts(request_id) {
            for ex in &extracts {
                match jsonpath_lib::select(&json, &ex.jsonpath) {
                    Ok(vals) if !vals.is_empty() => {
                        let val = serde_json::to_string(&vals[0])
                            .unwrap_or_default()
                            .trim_matches('"')
                            .to_string();
                        let _ = state.db.upsert_variable(req.project_id, &ex.target_var_key, &val);
                    }
                    _ => {}
                }
            }
        }
    }

    // Notify UI that variables may have changed
    let _ = app.emit("variables-changed", req.project_id);

    Ok(ResponseData {
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("").to_string(),
        time_ms,
        size_bytes: body.len() as u64,
        headers: resp_headers,
        body,
    })
}

// ---------- cURL export ----------

#[tauri::command]
pub fn export_curl(state: State<AppState>, request_id: i64) -> Result<String, String> {
    let req = state
        .db
        .get_request(request_id)
        .map_err(|e| e.to_string())?
        .ok_or("Request not found".to_string())?;
    let project_vars = state
        .db
        .list_variables(req.project_id)
        .map_err(|e| e.to_string())?;
    let vars = vars_map(&project_vars);

    let headers = parse_headers_json(&req.headers_json);
    let auth = parse_auth_json(&req.auth_json);

    let mut cmd = vec!["curl".to_string()];
    if req.method != "GET" {
        cmd.push("-X".into());
        cmd.push(req.method.to_uppercase());
    }
    cmd.push(format!("'{}'", resolve(&req.url, &vars)));

    for h in &headers {
        if h.key.is_empty() {
            continue;
        }
        cmd.push("-H".into());
        cmd.push(format!("'{}: {}'", h.key, resolve(&h.value, &vars)));
    }

    match auth.type_ {
        AuthType::Bearer => {
            let token = resolve(&auth.token, &vars);
            if !token.is_empty() {
                cmd.push("-H".into());
                cmd.push(format!("'Authorization: Bearer {}'", token));
            }
        }
        AuthType::Basic => {
            let user = resolve(&auth.username, &vars);
            let pass = resolve(&auth.password, &vars);
            cmd.push("-u".into());
            cmd.push(format!("'{}:{}'", user, pass));
        }
        AuthType::ApiKey => {
            let key = resolve(&auth.key, &vars);
            let value = resolve(&auth.value, &vars);
            if !key.is_empty() && auth.in_header {
                cmd.push("-H".into());
                cmd.push(format!("'{}: {}'", key, value));
            }
        }
        AuthType::None => {}
    }

    if !req.body.is_empty() {
        let body = resolve(&req.body, &vars);
        match req.body_type.as_str() {
            "json" => {
                cmd.push("--data-raw".into());
                cmd.push(format!("'{}'", body));
            }
            "form" | "form-data" => {
                for line in body.lines() {
                    if let Some((k, v)) = line.split_once('=') {
                        cmd.push("-F".into());
                        cmd.push(format!("'{}={}'", k, v));
                    }
                }
            }
            _ => {
                cmd.push("--data-raw".into());
                cmd.push(format!("'{}'", body));
            }
        }
    }

    Ok(cmd.join(" \\\n  "))
}

// ---------- cURL import ----------

fn tokenize_curl(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut in_single = false;
    let mut in_double = false;
    for c in input.chars() {
        match c {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            ' ' | '\t' | '\n' | '\r' if !in_single && !in_double => {
                if !cur.is_empty() {
                    tokens.push(std::mem::take(&mut cur));
                }
            }
            '\\' if !in_single && !in_double => {
                // line continuation: skip, next char handled normally
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        tokens.push(cur);
    }
    tokens
}

#[tauri::command]
pub fn parse_curl(input: String) -> Result<CurlImportResult, String> {
    let tokens = tokenize_curl(&input);
    let mut method = "GET".to_string();
    let mut url = String::new();
    let mut headers: Vec<Header> = Vec::new();
    let mut body_parts: Vec<String> = Vec::new();
    let mut body_type = "raw".to_string();
    let mut auth = Auth::default();
    let mut warnings: Vec<String> = Vec::new();

    let known_value_flags = [
        "-H", "--header", "-d", "--data", "--data-raw", "--data-binary", "--json", "-u", "--user",
        "-X", "--request", "--url", "-A", "--user-agent", "--max-time", "-m",
    ];

    let mut i = 0;
    while i < tokens.len() {
        let tok = &tokens[i];
        match tok.as_str() {
            "curl" => {}
            "-H" | "--header" => {
                i += 1;
                if i >= tokens.len() {
                    break;
                }
                if let Some((k, v)) = tokens[i].split_once(':') {
                    headers.push(Header {
                        key: k.trim().to_string(),
                        value: v.trim().to_string(),
                    });
                } else {
                    warnings.push(format!("Malformed header: {}", tokens[i]));
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" => {
                i += 1;
                if i >= tokens.len() {
                    break;
                }
                body_parts.push(tokens[i].clone());
                body_type = "raw".into();
            }
            "--json" => {
                i += 1;
                if i >= tokens.len() {
                    break;
                }
                body_parts.push(tokens[i].clone());
                body_type = "json".into();
            }
            "-u" | "--user" => {
                i += 1;
                if i >= tokens.len() {
                    break;
                }
                if let Some((user, pass)) = tokens[i].split_once(':') {
                    auth.type_ = AuthType::Basic;
                    auth.username = user.to_string();
                    auth.password = pass.to_string();
                }
            }
            "-X" | "--request" => {
                i += 1;
                if i >= tokens.len() {
                    break;
                }
                method = tokens[i].to_uppercase();
            }
            "--url" => {
                i += 1;
                if i >= tokens.len() {
                    break;
                }
                url = tokens[i].clone();
            }
            "-F" | "--form" => {
                warnings.push("Multipart (-F) is not supported — add fields manually in the UI".into());
                i += 1;
                if i < tokens.len() && !tokens[i].starts_with('-') {
                    // consume value, convert to form field
                    body_parts.push(tokens[i].clone());
                    body_type = "form-data".into();
                }
            }
            t if t.starts_with('-') => {
                warnings.push(format!("Unknown flag ignored: {}", t));
                // flags that take a value: skip next token if it's not a flag
                let takes_value = known_value_flags.contains(&t)
                    || matches!(t, "-o" | "--output" | "--connect-timeout" | "-b" | "--cookie");
                if takes_value && i + 1 < tokens.len() && !tokens[i + 1].starts_with('-') {
                    i += 1;
                }
            }
            t if url.is_empty() && (t.starts_with("http://") || t.starts_with("https://")) => {
                url = t.to_string();
            }
            _ => {}
        }
        i += 1;
    }

    // Detect Authorization header -> auth
    let mut remaining_headers = Vec::new();
    for h in headers {
        if h.key.eq_ignore_ascii_case("authorization") {
            if let Some(rest) = h.value.strip_prefix("Bearer ") {
                if matches!(auth.type_, AuthType::None) {
                    auth.type_ = AuthType::Bearer;
                    auth.token = rest.to_string();
                    continue;
                }
            } else if let Some(rest) = h.value.strip_prefix("Basic ") {
                if matches!(auth.type_, AuthType::None) {
                    if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(rest) {
                        if let Ok(s) = String::from_utf8(decoded) {
                            if let Some((u, p)) = s.split_once(':') {
                                auth.type_ = AuthType::Basic;
                                auth.username = u.to_string();
                                auth.password = p.to_string();
                                continue;
                            }
                        }
                    }
                }
            }
        }
        remaining_headers.push(h);
    }

    let body = body_parts.join("\n");
    if !body.is_empty() && method == "GET" {
        method = "POST".into();
    }

    Ok(CurlImportResult {
        method,
        url,
        headers: remaining_headers,
        body_type,
        body,
        auth,
        warnings,
    })
}

// ---------- Project export / import (.poster.json) ----------

#[derive(Serialize)]
struct ExportFile {
    version: u32,
    name: String,
    exported_at: String,
    variables: Vec<serde_json::Value>,
    requests: Vec<serde_json::Value>,
}

fn validate_export(doc: &serde_json::Value) -> Result<(), String> {
    let obj = doc.as_object().ok_or("Root must be a JSON object")?;
    if !obj.get("version").and_then(|v| v.as_u64()).unwrap_or(0) >= 1 {
        return Err("Missing or invalid 'version' field".into());
    }
    if obj.get("name").and_then(|v| v.as_str()).is_none() {
        return Err("Missing 'name' field".into());
    }
    if let Some(reqs) = obj.get("requests") {
        if !reqs.is_array() {
            return Err("'requests' must be an array".into());
        }
        for (i, r) in reqs.as_array().unwrap().iter().enumerate() {
            if r.get("method").and_then(|v| v.as_str()).is_none() {
                return Err(format!("requests[{}] missing 'method'", i));
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn export_project(state: State<AppState>, project_id: i64) -> Result<String, String> {
    let project = state
        .db
        .list_projects()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|p| p.id == project_id)
        .ok_or("Project not found".to_string())?;
    let requests = state
        .db
        .list_requests(project_id)
        .map_err(|e| e.to_string())?;
    let variables = state
        .db
        .list_variables(project_id)
        .map_err(|e| e.to_string())?;

    let vars_json: Vec<serde_json::Value> = variables
        .iter()
        .map(|v| serde_json::json!({ "key": v.key, "value": v.value }))
        .collect();

    let reqs_json: Vec<serde_json::Value> = requests
        .iter()
        .map(|r| {
            let extracts = state.db.list_extracts(r.id).unwrap_or_default();
            serde_json::json!({
                "name": r.name,
                "method": r.method,
                "url": r.url,
                "headers": parse_headers_json(&r.headers_json),
                "body_type": r.body_type,
                "body": r.body,
                "auth": parse_auth_json(&r.auth_json),
                "extracts": extracts.iter().map(|e| serde_json::json!({
                    "source_request_id": serde_json::json!(null),
                    "jsonpath": e.jsonpath,
                    "target_var_key": e.target_var_key,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();

    let file = ExportFile {
        version: 1,
        name: project.name,
        exported_at: db::now_iso(),
        variables: vars_json,
        requests: reqs_json,
    };
    serde_json::to_string_pretty(&file).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_project(
    state: State<AppState>,
    content: String,
    mode: String,
    target_project_id: Option<i64>,
) -> Result<Project, String> {
    let doc: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Invalid JSON: {}", e))?;
    validate_export(&doc)?;

    let name = doc["name"].as_str().unwrap_or("Imported").to_string();
    let vars: Vec<(String, String)> = doc["variables"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    Some((
                        v["key"].as_str()?.to_string(),
                        v["value"].as_str().unwrap_or("").to_string(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    let requests = doc
        .get("requests")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    match mode.as_str() {
        "new" => state.db.import_project(&name, &vars, &requests).map_err(|e| e.to_string()),
        "append" => {
            let pid = target_project_id.ok_or("target_project_id required for append")?;
            state
                .db
                .append_requests_to_project(pid, &vars, &requests)
                .map_err(|e| e.to_string())?;
            state
                .db
                .list_projects()
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|p| p.id == pid)
                .ok_or("Target project not found".to_string())
        }
        _ => Err("Unknown import mode".into()),
    }
}

// ---------- Load test control ----------

#[tauri::command]
pub async fn start_load_test(
    app: AppHandle,
    state: State<'_, AppState>,
    config: LoadTestConfig,
) -> Result<String, String> {
    let id = uuid_like();
    // Hand the same map to the spawned tasks so the handle is registered before
    // any worker can finish.
    spawn_load_test(app, state.load_tests.clone(), id.clone(), config).await?;
    Ok(id)
}

#[tauri::command]
pub fn stop_load_test(state: State<'_, AppState>, load_test_id: String) -> Result<(), String> {
    let mut map = state.load_tests.lock().unwrap();
    match map.get_mut(&load_test_id) {
        Some(h) => {
            // Signal cancellation; the collector task will emit the final report and
            // we drop the handle once it's done (tracked via a completion flag).
            h.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        }
        None => Err("Load test not found".into()),
    }
}

#[tauri::command]
pub fn cleanup_load_test(state: State<'_, AppState>, load_test_id: String) -> Result<(), String> {
    state.load_tests.lock().unwrap().remove(&load_test_id);
    Ok(())
}

#[tauri::command]
pub fn load_test_status(state: State<'_, AppState>, load_test_id: String) -> Result<LoadTestStats, String> {
    let map = state.load_tests.lock().unwrap();
    match map.get(&load_test_id) {
        Some(h) => Ok(h.stats()),
        None => Err("Load test not found".into()),
    }
}

fn uuid_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}-{:x}", t, fastrand::u64(0..u64::MAX))
}

// ---------- File I/O (for export/import + reports) ----------

#[tauri::command]
pub fn save_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn read_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}


