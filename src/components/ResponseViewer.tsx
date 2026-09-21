import { useMemo, useState } from "react";
import type { ResponseData } from "../types";

interface Props {
  response: ResponseData | null;
}

function fmtBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}

function JsonNode({ name, value }: { name: string | null; value: unknown }) {
  const [open, setOpen] = useState(true);

  if (value === null) return <span className="json-null">null</span>;
  if (typeof value === "string") return <span className="json-string">"{value}"</span>;
  if (typeof value === "number" || typeof value === "boolean") {
    return <span className={typeof value === "number" ? "json-number" : "json-bool"}>{String(value)}</span>;
  }

  const isArr = Array.isArray(value);
  const entries: [string, unknown][] = isArr
    ? (value as unknown[]).map((v, i) => [String(i), v])
    : Object.entries(value as Record<string, unknown>);

  return (
    <div>
      <span onClick={() => setOpen(!open)} style={{ cursor: "pointer", userSelect: "none" }}>
        {isArr ? "[" : "{"}
      </span>{" "}
      {open && entries.length > 0 && (
        <>
          {"\n"}
          {entries.map(([k, v], i) => (
            <div key={i}>
              {"  ".repeat(1)}
              {name !== null || true ? (
                <>
                  <span className="json-key">"{k}"</span>:{" "}
                  <JsonNode name={k} value={v} />
                </>
              ) : null}
            </div>
          ))}
          {"\n"}
        </>
      )}
      <span onClick={() => setOpen(!open)} style={{ cursor: "pointer", userSelect: "none" }}>
        {isArr ? "]" : "}"}
      </span>
    </div>
  );
}

export default function ResponseViewer({ response }: Props) {
  const [tab, setTab] = useState<"body" | "headers">("body");
  const [view, setView] = useState<"tree" | "raw">("tree");

  const parsed = useMemo(() => {
    if (!response) return null;
    try {
      return JSON.parse(response.body);
    } catch {
      return undefined;
    }
  }, [response]);

  if (!response) {
    return (
      <div className="response-area">
        <div className="empty-state">Send a request to see the response</div>
      </div>
    );
  }

  const statusClass =
    response.status >= 200 && response.status < 300
      ? "status-2xx"
      : response.status >= 300 && response.status < 400
        ? "status-3xx"
        : response.status >= 400
          ? "status-4xx"
          : "status-other";

  return (
    <div className="response-area">
      <div className="response-meta">
        <span className={`status-pill ${statusClass}`}>
          {response.status} {response.status_text}
        </span>
        <span>{(response.time_ms / 1000).toFixed(2)} s</span>
        <span>{fmtBytes(response.size_bytes)}</span>
        <div style={{ flex: 1 }} />
        {tab === "body" && (
          <>
            <button className={`icon-btn ${view === "tree" ? "" : ""}`} onClick={() => setView("tree")}>JSON</button>
            <button className="icon-btn" onClick={() => setView("raw")}>Raw</button>
          </>
        )}
        {tab === "body" && (
          <>
            <span style={{ opacity: 0.3 }}>|</span>
          </>
        )}
        <button className={`icon-btn ${tab === "body" ? "" : ""}`} onClick={() => setTab("body")}>Body</button>
        <button className="icon-btn" onClick={() => setTab("headers")}>Headers ({response.headers.length})</button>
      </div>

      <div className="response-body">
        {tab === "headers" ? (
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
            <tbody>
              {response.headers.map(([k, v], i) => (
                <tr key={i} style={{ borderBottom: "1px solid #333340" }}>
                  <td style={{ padding: "3px 8px", color: "#8fa6ff", whiteSpace: "nowrap" }}>{k}</td>
                  <td style={{ padding: "3px 8px", wordBreak: "break-all" }}>{v}</td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : view === "raw" || parsed === undefined ? (
          <pre className="raw-body">{response.body || "(empty body)"}</pre>
        ) : (
          <div className="json-tree">
            <JsonNode name={null} value={parsed} />
          </div>
        )}
      </div>
    </div>
  );
}
