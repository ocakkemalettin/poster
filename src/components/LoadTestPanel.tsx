import { useState } from "react";
import type { LoadTestReport, LoadTestStats, Request, Variable, VarOverride } from "../types";

interface Props {
  request: Request;
  variables: Variable[];
  runningId: string | null;
  stats: LoadTestStats | null;
  report: LoadTestReport | null;
  onStart: (cfg: { rps: number; total_requests: number; concurrency: number; timeout_ms: number; overrides: VarOverride[] }) => void;
  onStop: () => void;
}

export default function LoadTestPanel({ request, variables, runningId, stats, report, onStart, onStop }: Props) {
  const [rps, setRps] = useState(10);
  const [total, setTotal] = useState(100);
  const [concurrency, setConcurrency] = useState(8);
  const [timeoutMs, setTimeoutMs] = useState(5000);
  const [overrides, setOverrides] = useState<VarOverride[]>([]);

  function updateOverride(i: number, patch: Partial<VarOverride>) {
    const next = [...overrides];
    next[i] = { ...next[i], ...patch };
    setOverrides(next);
  }

  return (
    <div className="lt-content">
      <p className="hint">
        Load test for <b>{request.name}</b> ({request.method} {request.url}). Variables can be overridden per-run.
      </p>

      <div className="lt-field">
        <label>RPS</label>
        <input type="number" min={1} value={rps} onChange={(e) => setRps(Number(e.target.value))} />
      </div>
      <div className="lt-field">
        <label>Total requests</label>
        <input type="number" min={1} value={total} onChange={(e) => setTotal(Number(e.target.value))} />
      </div>
      <div className="lt-field">
        <label>Concurrency</label>
        <input type="number" min={1} max={256} value={concurrency} onChange={(e) => setConcurrency(Number(e.target.value))} />
      </div>
      <div className="lt-field">
        <label>Timeout (ms)</label>
        <input type="number" min={100} value={timeoutMs} onChange={(e) => setTimeoutMs(Number(e.target.value))} />
      </div>

      <p className="hint" style={{ marginTop: 8 }}>Variable overrides</p>
      {variables.length === 0 && <p className="hint">No variables in this project.</p>}
      <div className="override-table">
        {overrides.map((o, i) => (
          <div key={i} style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <div className="override-row">
              <input value={o.key} onChange={(e) => updateOverride(i, { key: e.target.value })} placeholder="{{var}}" />
              <select value={o.mode} onChange={(e) => updateOverride(i, { mode: e.target.value as VarOverride["mode"] })}>
                <option value="fixed">fixed</option>
                <option value="range">range</option>
                <option value="list">list</option>
              </select>
            </div>
            <div className="override-values">
              <input
                value={o.value}
                onChange={(e) => updateOverride(i, { value: e.target.value })}
                placeholder={o.mode === "range" ? "start" : o.mode === "list" ? "a,b,c" : "value"}
              />
              {o.mode === "range" ? (
                <input value={o.value_end ?? ""} onChange={(e) => updateOverride(i, { value_end: e.target.value })} placeholder="end" />
              ) : (
                <button className="icon-btn" onClick={() => setOverrides(overrides.filter((_, j) => j !== i))}>✕</button>
              )}
            </div>
          </div>
        ))}
      </div>
      {variables.length > 0 && (
        <select
          value=""
          onChange={(e) => {
            if (!e.target.value) return;
            setOverrides([...overrides, { key: e.target.value, mode: "fixed", value: "" }]);
          }}
        >
          <option value="">+ Add variable override…</option>
          {variables.map((v) => (
            <option key={v.key} value={v.key}>{v.key}</option>
          ))}
        </select>
      )}

      <div style={{ display: "flex", gap: 8, marginTop: 10 }}>
        {!runningId ? (
          <button className="start-btn" onClick={() => onStart({ rps, total_requests: total, concurrency, timeout_ms: timeoutMs, overrides })}>
            Start load test
          </button>
        ) : (
          <button className="stop-btn" onClick={onStop}>Stop</button>
        )}
      </div>

      {stats && runningId && (
        <div className="stats-grid" style={{ marginTop: 12 }}>
          <StatBox num={`${stats.completed}`} lbl={`of ${total}`} />
          <StatBox num={`${stats.successes}`} lbl="success" color="#68d391" />
          <StatBox num={`${stats.failures}`} lbl="failures" color="#fc8181" />
          <StatBox num={`${stats.avg_ms.toFixed(0)} ms`} lbl="avg latency" />
          <StatBox num={`${stats.p95_ms.toFixed(0)} ms`} lbl="p95" />
          <StatBox num={`${(stats.elapsed_ms / 1000).toFixed(1)} s`} lbl="elapsed" />
        </div>
      )}

      {report && !runningId && (
        <>
          <h4 style={{ margin: "12px 0 6px" }}>Report</h4>
          <div className="stats-grid">
            <StatBox num={`${report.total}`} lbl="total" />
            <StatBox num={`${report.successes}`} lbl="success" color="#68d391" />
            <StatBox num={`${report.failures}`} lbl="failures" color="#fc8181" />
            <StatBox num={`${(report.duration_ms / 1000).toFixed(1)} s`} lbl="duration" />
            <StatBox num={`${report.avg_ms.toFixed(0)} ms`} lbl="avg" />
            <StatBox num={`${report.rps_actual.toFixed(1)}`} lbl="actual RPS" />
          </div>
        </>
      )}
    </div>
  );
}

function StatBox({ num, lbl, color }: { num: string; lbl: string; color?: string }) {
  return (
    <div className="stat-box">
      <div className="num" style={color ? { color } : undefined}>{num}</div>
      <div className="lbl">{lbl}</div>
    </div>
  );
}
