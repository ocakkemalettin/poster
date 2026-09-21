import { useEffect, useState } from "react";
import type { Auth, Extract, Header, Request, Variable } from "../types";
import { authToJson, headersToJson, parseAuth, parseHeaders } from "../api";

interface Props {
  request: Request;
  variables: Variable[];
  extracts: Extract[];
  requestsInProject: Request[];
  sending: boolean;
  curlWarnings: string[] | null;
  onSave: (r: Request) => void;
  onSend: () => void;
  onImportCurl: (text: string) => void;
  onExportCurl: () => void;
  onAddExtract: (e: Omit<Extract, "id" | "request_id">) => void;
  onDeleteExtract: (id: number) => void;
}

const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"];
type Tab = "params" | "headers" | "auth" | "body" | "extracts";

export default function RequestEditor({
  request,
  variables,
  extracts,
  requestsInProject,
  sending,
  curlWarnings,
  onSave,
  onSend,
  onImportCurl,
  onExportCurl,
  onAddExtract,
  onDeleteExtract,
}: Props) {
  const [name, setName] = useState(request.name);
  const [method, setMethod] = useState(request.method);
  const [url, setUrl] = useState(request.url);
  const [headers, setHeaders] = useState<Header[]>([]);
  const [bodyType, setBodyType] = useState(request.body_type);
  const [body, setBody] = useState(request.body);
  const [auth, setAuth] = useState<Auth>(parseAuth(request.auth_json));
  const [tab, setTab] = useState<Tab>("params");

  // curl import textarea modal state
  const [showCurlImport, setShowCurlImport] = useState(false);
  const [curlText, setCurlText] = useState("");

  useEffect(() => {
    setName(request.name);
    setMethod(request.method);
    setUrl(request.url);
    setHeaders(parseHeaders(request.headers_json));
    setBodyType(request.body_type);
    setBody(request.body);
    setAuth(parseAuth(request.auth_json));
  }, [request.id]); // eslint-disable-line react-hooks/exhaustive-deps

  function persist() {
    onSave({
      ...request,
      name: name.trim() || "Request",
      method,
      url,
      headers_json: headersToJson(headers),
      body_type: bodyType,
      body,
      auth_json: authToJson(auth),
    });
  }

  function updateHeader(i: number, field: "key" | "value", val: string) {
    const next = [...headers];
    next[i] = { ...next[i], [field]: val };
    setHeaders(next);
  }

  // query params derived from URL
  let params: { key: string; value: string }[] = [];
  try {
    if (url.includes("?")) {
      const qs = new URL(url.startsWith("http") ? url : "http://x" + url).searchParams;
      params = [...qs.entries()].map(([key, value]) => ({ key, value }));
    }
  } catch {
    /* invalid url */
  }

  function setParam(i: number, field: "key" | "value", val: string) {
    const base = url.split("?")[0];
    const qs = new URLSearchParams();
    params.forEach((p, idx) => {
      if (idx !== i) qs.append(p.key, p.value);
    });
    if (val !== "" || field === "value") qs.append(params[i].key, val);
    setUrl(qs.toString() ? `${base}?${qs}` : base);
  }

  function addParam() {
    const base = url.split("?")[0];
    const qs = new URLSearchParams(url.includes("?") ? url.split("?")[1] : "");
    qs.append("", "");
    setUrl(`${base}?${qs}`);
  }

  return (
    <div className="editor-area">
      {curlWarnings && curlWarnings.length > 0 && (
        <div className="warn-box">{curlWarnings.join("\n")}</div>
      )}

      <input value={name} onChange={(e) => setName(e.target.value)} onBlur={persist} placeholder="Request name" />

      <div className="url-bar">
        <select value={method} onChange={(e) => { setMethod(e.target.value); persist(); }}>
          {METHODS.map((m) => (
            <option key={m}>{m}</option>
          ))}
        </select>
        <input
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          onBlur={persist}
          onKeyDown={(e) => e.key === "Enter" && onSend()}
          placeholder="https://api.example.com/path  ({{variables}} supported)"
        />
        <button className="send-btn" disabled={sending || !url.trim()} onClick={onSend}>
          {sending ? "Sending…" : "Send"}
        </button>
      </div>

      <div style={{ display: "flex", gap: 6 }}>
        <button className="icon-btn" onClick={() => setShowCurlImport(true)}>Import cURL</button>
        <button className="icon-btn" onClick={onExportCurl}>Copy as cURL</button>
      </div>

      <div className="tabs">
        {(["params", "headers", "auth", "body", "extracts"] as Tab[]).map((t) => (
          <button key={t} className={`tab ${tab === t ? "active" : ""}`} onClick={() => setTab(t)}>
            {t === "params" ? `Params (${params.length})` : t === "headers" ? `Headers (${headers.filter((h) => h.key).length})` : t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      <div className="tab-content">
        {tab === "params" && (
          <>
            {params.map((p, i) => (
              <div key={i} className="kv-row">
                <input value={p.key} placeholder="key" onChange={(e) => setParam(i, "key", e.target.value)} />
                <input value={p.value} placeholder="value" onChange={(e) => setParam(i, "value", e.target.value)} />
                <button className="icon-btn" onClick={() => setParam(i, "value", "")}>✕</button>
              </div>
            ))}
            <button className="icon-btn" onClick={addParam}>+ Add parameter</button>
          </>
        )}

        {tab === "headers" && (
          <>
            {headers.map((h, i) => (
              <div key={i} className="kv-row">
                <input value={h.key} placeholder="key" onChange={(e) => updateHeader(i, "key", e.target.value)} onBlur={persist} />
                <input value={h.value} placeholder="value" onChange={(e) => updateHeader(i, "value", e.target.value)} onBlur={persist} />
                <button className="icon-btn" onClick={() => { setHeaders(headers.filter((_, j) => j !== i)); persist(); }}>✕</button>
              </div>
            ))}
            <button
              className="icon-btn"
              onClick={() => { setHeaders([...headers, { key: "", value: "" }]); }}
            >
              + Add header
            </button>
          </>
        )}

        {tab === "auth" && (
          <>
            <div className="lt-field">
              <label>Type</label>
              <select
                value={auth.type_}
                onChange={(e) => { setAuth({ ...auth, type_: e.target.value as Auth["type_"] }); persist(); }}
              >
                <option value="none">None</option>
                <option value="bearer">Bearer Token</option>
                <option value="basic">Basic Auth</option>
                <option value="api_key">API Key</option>
              </select>
            </div>
            {auth.type_ === "bearer" && (
              <div className="lt-field">
                <label>Token</label>
                <input value={auth.token} onChange={(e) => setAuth({ ...auth, token: e.target.value })} onBlur={persist} placeholder="{{token}}" />
              </div>
            )}
            {auth.type_ === "basic" && (
              <>
                <div className="lt-field">
                  <label>Username</label>
                  <input value={auth.username} onChange={(e) => setAuth({ ...auth, username: e.target.value })} onBlur={persist} />
                </div>
                <div className="lt-field">
                  <label>Password</label>
                  <input type="password" value={auth.password} onChange={(e) => setAuth({ ...auth, password: e.target.value })} onBlur={persist} />
                </div>
              </>
            )}
            {auth.type_ === "api_key" && (
              <>
                <div className="lt-field">
                  <label>Key</label>
                  <input value={auth.key} onChange={(e) => setAuth({ ...auth, key: e.target.value })} onBlur={persist} placeholder="X-API-Key" />
                </div>
                <div className="lt-field">
                  <label>Value</label>
                  <input value={auth.value} onChange={(e) => setAuth({ ...auth, value: e.target.value })} onBlur={persist} />
                </div>
                <div className="lt-field">
                  <label>In</label>
                  <select value={auth.in_header ? "header" : "query"} onChange={(e) => { setAuth({ ...auth, in_header: e.target.value === "header" }); persist(); }}>
                    <option value="header">Header</option>
                    <option value="query">Query Param</option>
                  </select>
                </div>
              </>
            )}
          </>
        )}

        {tab === "body" && (
          <>
            <select className="body-type-select" value={bodyType} onChange={(e) => { setBodyType(e.target.value); persist(); }}>
              <option value="none">None</option>
              <option value="json">JSON</option>
              <option value="form">Form (x-www-form-urlencoded)</option>
              <option value="form-data">Multipart form-data</option>
              <option value="raw">Raw text</option>
            </select>
            {bodyType !== "none" && (
              <>
                {(bodyType === "form" || bodyType === "form-data") ? (
                  <textarea
                    className="body-editor"
                    value={body}
                    onChange={(e) => setBody(e.target.value)}
                    onBlur={persist}
                    placeholder={"key=value\none per line"}
                  />
                ) : (
                  <textarea
                    className="body-editor"
                    value={body}
                    onChange={(e) => setBody(e.target.value)}
                    onBlur={persist}
                    placeholder={bodyType === "json" ? '{"key": "value"}' : ""}
                  />
                )}
                {bodyType === "json" && (
                  <button className="icon-btn" onClick={() => { try { setBody(JSON.stringify(JSON.parse(body), null, 2)); } catch { alert("Invalid JSON"); } }}>
                    Pretty print
                  </button>
                )}
              </>
            )}
          </>
        )}

        {tab === "extracts" && (
          <>
            <p className="hint">After sending, these JSONPath expressions are evaluated against the response and stored into project variables.</p>
            {extracts.map((e) => (
              <div key={e.id} className="kv-row">
                <input value={e.jsonpath} readOnly placeholder='$.data.token' />
                <input value={e.target_var_key} readOnly placeholder="var name" />
                <button className="icon-btn" onClick={() => onDeleteExtract(e.id)}>✕</button>
              </div>
            ))}
            <AddExtractForm
              variables={variables.map((v) => v.key)}
              requestsInProject={requestsInProject.filter((r) => r.id !== request.id)}
              onRequest={onAddExtract}
            />
          </>
        )}
      </div>

      {showCurlImport && (
        <div style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,.6)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 10 }}>
          <div style={{ background: "#2a2a35", borderRadius: 8, padding: 16, width: 560 }}>
            <h3 style={{ marginTop: 0 }}>Import cURL</h3>
            <textarea
              value={curlText}
              onChange={(e) => setCurlText(e.target.value)}
              placeholder={"curl -X POST 'https://api.example.com/login' \\\n  -H 'Content-Type: application/json' \\\n  --data '{\"user\":\"a\"}'"}
              style={{ width: "100%", minHeight: 160 }}
            />
            <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <button className="icon-btn" onClick={() => setShowCurlImport(false)}>Cancel</button>
              <button className="send-btn" disabled={!curlText.trim()} onClick={() => { onImportCurl(curlText); setShowCurlImport(false); setCurlText(""); }}>
                Import
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function AddExtractForm({
  variables,
  requestsInProject,
  onRequest,
}: {
  variables: string[];
  requestsInProject: Request[];
  onRequest: (e: Omit<Extract, "id" | "request_id">) => void;
}) {
  const [jsonpath, setJsonpath] = useState("");
  const [target, setTarget] = useState(variables[0] ?? "");
  const [sourceId, setSourceId] = useState<number | null>(null);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      <input value={jsonpath} onChange={(e) => setJsonpath(e.target.value)} placeholder="JSONPath e.g. $.data.token" />
      <select value={target} onChange={(e) => setTarget(e.target.value)}>
        {variables.map((v) => (
          <option key={v}>{v}</option>
        ))}
        {!variables.length && <option value="">(no variables — add one in Variables panel)</option>}
      </select>
      <select value={sourceId ?? ""} onChange={(e) => setSourceId(e.target.value === "" ? null : Number(e.target.value))}>
        <option value="">Response of this request</option>
        {requestsInProject.map((r) => (
          <option key={r.id} value={r.id}>{r.name}</option>
        ))}
      </select>
      <button
        className="icon-btn"
        disabled={!jsonpath.trim() || !target}
        onClick={() => {
          onRequest({ source_request_id: sourceId, jsonpath: jsonpath.trim(), target_var_key: target });
          setJsonpath("");
        }}
      >
        + Add extract
      </button>
    </div>
  );
}
