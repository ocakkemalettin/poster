import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { Extract, LoadTestReport, LoadTestStats, Project, Request, ResponseData, Variable } from "./types";
import * as api from "./api";
import ProjectTree from "./components/ProjectTree";
import RequestEditor from "./components/RequestEditor";
import ResponseViewer from "./components/ResponseViewer";
import VariablesPanel from "./components/VariablesPanel";
import LoadTestPanel from "./components/LoadTestPanel";
import "./App.css";

export default function App() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [requests, setRequests] = useState<Request[]>([]);
  const [variables, setVariables] = useState<Variable[]>([]);
  const [extracts, setExtracts] = useState<Extract[]>([]);

  const [activeProjectId, setActiveProjectId] = useState<number | null>(null);
  const [activeRequestId, setActiveRequestId] = useState<number | null>(null);

  const [response, setResponse] = useState<ResponseData | null>(null);
  const [sending, setSending] = useState(false);
  const [curlWarnings, setCurlWarnings] = useState<string[] | null>(null);

  const [rightTab, setRightTab] = useState<"variables" | "loadtest">("variables");

  const [runningId, setRunningId] = useState<string | null>(null);
  const runningIdRef = useRef<string | null>(null);
  const [stats, setStats] = useState<LoadTestStats | null>(null);
  const [report, setReport] = useState<LoadTestReport | null>(null);

  function setRunning(id: string | null) {
    runningIdRef.current = id;
    setRunningId(id);
  }

  const activeProject = projects.find((p) => p.id === activeProjectId) ?? null;
  const activeRequest = requests.find((r) => r.id === activeRequestId) ?? null;

  // ---------- data loading ----------

  const refreshProjects = useCallback(async () => {
    setProjects(await api.listProjects());
  }, []);

  const loadProjectData = useCallback(
    async (pid: number) => {
      const [reqs, vars] = await Promise.all([api.listRequests(pid), api.listVariables(pid)]);
      setRequests(reqs);
      setVariables(vars);
    },
    [],
  );

  const loadRequestData = useCallback(async (rid: number) => {
    setExtracts(await api.listExtracts(rid));
  }, []);

  useEffect(() => {
    refreshProjects().then(() => {});
  }, [refreshProjects]);

  // auto-select first project on startup
  useEffect(() => {
    if (projects.length > 0 && activeProjectId === null) {
      setActiveProjectId(projects[0].id);
    }
  }, [projects, activeProjectId]);

  useEffect(() => {
    if (activeProjectId !== null) loadProjectData(activeProjectId);
  }, [activeProjectId, loadProjectData]);

  useEffect(() => {
    if (activeRequestId !== null) loadRequestData(activeRequestId);
    else setExtracts([]);
  }, [activeRequestId, loadRequestData]);

  // ---------- events from Rust ----------

  useEffect(() => {
    const un1 = listen<number>("variables-changed", async (e) => {
      if (e.payload === activeProjectId) await loadProjectData(activeProjectId);
    });
    const un2 = listen<LoadTestStats>("load-test-stats", (e) => setStats(e.payload));
    const un3 = listen<LoadTestReport>("load-test-finished", async (e) => {
      setReport(e.payload);
      if (runningIdRef.current) await api.cleanupLoadTest(runningIdRef.current);
      setRunning(null);
      setStats(null);
      api.downloadReport(e.payload);
    });
    return () => {
      void un1.then((u) => u());
      void un2.then((u) => u());
      void un3.then((u) => u());
    };
  }, [activeProjectId, loadProjectData]);

  // ---------- project handlers ----------

  const handleSelectProject = (id: number) => {
    setActiveProjectId(id);
    setResponse(null);
    setCurlWarnings(null);
  };

  const handleCreateProject = async () => {
    const name = prompt("Project name", "New Project");
    if (!name?.trim()) return;
    const p = await api.createProject(name.trim());
    await refreshProjects();
    setActiveProjectId(p.id);
  };

  const handleRenameProject = async (id: number, name: string) => {
    await api.renameProject(id, name);
    await refreshProjects();
  };

  const handleDeleteProject = async (id: number) => {
    await api.deleteProject(id);
    if (activeProjectId === id) {
      setActiveProjectId(null);
      setActiveRequestId(null);
      setRequests([]);
      setVariables([]);
      setResponse(null);
    }
    await refreshProjects();
  };

  // ---------- request handlers ----------

  const handleSelectRequest = (id: number) => {
    setActiveRequestId(id);
    setResponse(null);
    setCurlWarnings(null);
  };

  const handleCreateRequest = async () => {
    if (!activeProjectId) return;
    const name = prompt("Request name", "New Request");
    if (!name?.trim()) return;
    const r = await api.createRequest(activeProjectId, name.trim());
    await loadProjectData(activeProjectId);
    setActiveRequestId(r.id);
  };

  const handleSaveRequest = async (r: Request) => {
    await api.saveRequest(r);
    setRequests((prev) => prev.map((x) => (x.id === r.id ? r : x)));
  };

  const handleDeleteRequest = async (id: number) => {
    if (!activeProjectId) return;
    await api.deleteRequest(id);
    if (activeRequestId === id) setActiveRequestId(null);
    await loadProjectData(activeProjectId);
  };

  // ---------- send / curl ----------

  const handleSend = async () => {
    if (!activeRequestId) return;
    setSending(true);
    try {
      const res = await api.sendRequest(activeRequestId);
      setResponse(res);
    } catch (e) {
      alert(`Request failed: ${String(e)}`);
    } finally {
      setSending(false);
    }
  };

  const handleImportCurl = async (text: string) => {
    if (!activeRequestId) return;
    try {
      const parsed = await api.parseCurl(text);
      setCurlWarnings(parsed.warnings.length ? parsed.warnings : null);
      await handleSaveRequest({
        ...activeRequest!,
        method: parsed.method,
        url: parsed.url,
        headers_json: JSON.stringify(parsed.headers),
        body_type: parsed.body_type,
        body: parsed.body,
        auth_json: JSON.stringify(parsed.auth),
      });
    } catch (e) {
      alert(`cURL parse failed: ${String(e)}`);
    }
  };

  const handleExportCurl = async () => {
    if (!activeRequestId) return;
    try {
      const curl = await api.exportCurl(activeRequestId);
      await navigator.clipboard.writeText(curl);
      alert("cURL copied to clipboard");
    } catch (e) {
      alert(`Export failed: ${String(e)}`);
    }
  };

  // ---------- extracts ----------

  const handleAddExtract = async (e: Omit<Extract, "id" | "request_id">) => {
    if (!activeRequestId) return;
    await api.createExtract({ ...e, id: 0, request_id: activeRequestId });
    setExtracts(await api.listExtracts(activeRequestId));
  };

  const handleDeleteExtract = async (id: number) => {
    await api.deleteExtract(id);
    if (activeRequestId) setExtracts(await api.listExtracts(activeRequestId));
  };

  // ---------- variables ----------

  const handleUpsertVariable = async (key: string, value: string) => {
    if (!activeProjectId) return;
    await api.upsertVariable(activeProjectId, key, value);
    setVariables(await api.listVariables(activeProjectId));
  };

  const handleDeleteVariable = async (id: number) => {
    if (!activeProjectId) return;
    await api.deleteVariable(id);
    setVariables(await api.listVariables(activeProjectId));
  };

  // ---------- project export / import ----------

  const handleExportProject = async () => {
    if (!activeProjectId) return;
    try {
      const json = await api.exportProject(activeProjectId);
      await api.saveFile(`${activeProject?.name ?? "project"}.poster.json`, json);
    } catch (e) {
      alert(`Export failed: ${String(e)}`);
    }
  };

  const handleImportProject = async () => {
    try {
      const content = await api.pickFile();
      if (!content) return;
      const mode = confirm("OK = import as NEW project\nCancel = append to current project") ? "new" : "append";
      const target = mode === "append" ? activeProjectId ?? undefined : undefined;
      const p = await api.importProject(content, mode, target);
      await refreshProjects();
      setActiveProjectId(p.id);
    } catch (e) {
      alert(`Import failed: ${String(e)}`);
    }
  };

  // ---------- load test ----------

  const handleStartLoadTest = async (cfg: { rps: number; total_requests: number; concurrency: number; timeout_ms: number; overrides: import("./types").VarOverride[] }) => {
    if (!activeRequest) return;
    try {
      const headers = JSON.parse(activeRequest.headers_json || "[]");
      const auth = JSON.parse(activeRequest.auth_json || "{}");
      const id = await api.startLoadTest({
        url: activeRequest.url,
        method: activeRequest.method,
        headers,
        body_type: activeRequest.body_type,
        body: activeRequest.body,
        auth,
        variables: variables.map((v) => [v.key, v.value] as [string, string]),
        overrides: cfg.overrides,
        rps: cfg.rps,
        total_requests: cfg.total_requests,
        concurrency: cfg.concurrency,
        timeout_ms: cfg.timeout_ms,
      });
      setRunning(id);
      setReport(null);
      setStats(null);
    } catch (e) {
      alert(`Failed to start load test: ${String(e)}`);
    }
  };

  const handleStopLoadTest = async () => {
    if (!runningId) return;
    await api.stopLoadTest(runningId);
  };

  // ---------- render ----------

  return (
    <div className="app">
      <div className="topbar">
        <span className="logo">Poster</span>
        <select value={activeProjectId ?? ""} onChange={(e) => handleSelectProject(Number(e.target.value))}>
          {projects.map((p) => (
            <option key={p.id} value={p.id}>{p.name}</option>
          ))}
          {!projects.length && <option value="">No projects</option>}
        </select>
        <div style={{ flex: 1 }} />
        <button className="icon-btn" onClick={handleImportProject}>Import .poster.json</button>
        <button className="icon-btn" disabled={!activeProjectId} onClick={handleExportProject}>Export project</button>
      </div>

      <ProjectTree
        projects={projects}
        requests={requests}
        activeProjectId={activeProjectId}
        activeRequestId={activeRequestId}
        onSelectProject={handleSelectProject}
        onSelectRequest={handleSelectRequest}
        onCreateProject={handleCreateProject}
        onRenameProject={handleRenameProject}
        onDeleteProject={handleDeleteProject}
        onCreateRequest={handleCreateRequest}
        onRenameRequest={async (id, name) => {
          const r = requests.find((x) => x.id === id);
          if (r) await handleSaveRequest({ ...r, name });
        }}
        onDeleteRequest={handleDeleteRequest}
      />

      <div className="center-panel">
        {activeRequest ? (
          <>
            <div style={{ flex: 1, overflowY: "auto" }}>
              <RequestEditor
                request={activeRequest}
                variables={variables}
                extracts={extracts}
                requestsInProject={requests}
                sending={sending}
                curlWarnings={curlWarnings}
                onSave={handleSaveRequest}
                onSend={handleSend}
                onImportCurl={handleImportCurl}
                onExportCurl={handleExportCurl}
                onAddExtract={handleAddExtract}
                onDeleteExtract={handleDeleteExtract}
              />
            </div>
            <ResponseViewer response={response} />
          </>
        ) : (
          <div className="empty-state">Select or create a request to get started</div>
        )}
      </div>

      <div className="right-panel">
        <div className="right-tabs">
          <button className={`right-tab ${rightTab === "variables" ? "active" : ""}`} onClick={() => setRightTab("variables")}>
            Variables ({variables.length})
          </button>
          <button className={`right-tab ${rightTab === "loadtest" ? "active" : ""}`} onClick={() => setRightTab("loadtest")}>
            Load Test
          </button>
        </div>
        {rightTab === "variables" ? (
          activeProjectId ? (
            <VariablesPanel variables={variables} onUpsert={handleUpsertVariable} onDelete={handleDeleteVariable} />
          ) : (
            <div className="empty-state">Select a project</div>
          )
        ) : activeRequest ? (
          <LoadTestPanel
            request={activeRequest}
            variables={variables}
            runningId={runningId}
            stats={stats}
            report={report}
            onStart={handleStartLoadTest}
            onStop={handleStopLoadTest}
          />
        ) : (
          <div className="empty-state">Select a request to load test</div>
        )}
      </div>
    </div>
  );
}
