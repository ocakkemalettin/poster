import { invoke } from "@tauri-apps/api/core";
import type {
  Auth,
  CurlImportResult,
  Extract,
  Header,
  LoadTestConfig,
  LoadTestReport,
  LoadTestStats,
  Project,
  Request,
  ResponseData,
  Variable,
} from "./types";

export const listProjects = () => invoke<Project[]>("list_projects");
export const createProject = (name: string) => invoke<Project>("create_project", { name });
export const renameProject = (projectId: number, name: string) =>
  invoke<void>("rename_project", { projectId, name });
export const deleteProject = (projectId: number) =>
  invoke<void>("delete_project", { projectId });

export const listRequests = (projectId: number) =>
  invoke<Request[]>("list_requests", { projectId });
export const createRequest = (projectId: number, name: string) =>
  invoke<Request>("create_request", { projectId, name });
export const saveRequest = (request: Request) => invoke<void>("save_request", { request });
export const deleteRequest = (requestId: number) =>
  invoke<void>("delete_request", { requestId });

export const listVariables = (projectId: number) =>
  invoke<Variable[]>("list_variables", { projectId });
export const upsertVariable = (projectId: number, key: string, value: string) =>
  invoke<void>("upsert_variable", { projectId, key, value });
export const deleteVariable = (variableId: number) =>
  invoke<void>("delete_variable", { variableId });

export const listExtracts = (requestId: number) =>
  invoke<Extract[]>("list_extracts", { requestId });
export const createExtract = (extract: Extract) =>
  invoke<Extract>("create_extract", { extract });
export const deleteExtract = (extractId: number) =>
  invoke<void>("delete_extract", { extractId });

export const sendRequest = (requestId: number) =>
  invoke<ResponseData>("send_request", { requestId });

export const exportCurl = (requestId: number) => invoke<string>("export_curl", { requestId });
export const parseCurl = (input: string) => invoke<CurlImportResult>("parse_curl", { input });

export const exportProject = (projectId: number) =>
  invoke<string>("export_project", { projectId });
export const importProject = (content: string, mode: "new" | "append", targetProjectId?: number) =>
  invoke<Project>("import_project", { content, mode, targetProjectId });

export const startLoadTest = (config: LoadTestConfig) =>
  invoke<string>("start_load_test", { config });
export const stopLoadTest = (loadTestId: string) =>
  invoke<void>("stop_load_test", { loadTestId });
export const cleanupLoadTest = (loadTestId: string) =>
  invoke<void>("cleanup_load_test", { loadTestId });
export const loadTestStatus = (loadTestId: string) =>
  invoke<LoadTestStats>("load_test_status", { loadTestId });

// ---------- helpers ----------

export function parseHeaders(json: string): Header[] {
  try {
    const v = JSON.parse(json);
    return Array.isArray(v) ? v : [];
  } catch {
    return [];
  }
}

export function headersToJson(headers: Header[]): string {
  return JSON.stringify(
    headers.filter((h) => h.key.trim() !== ""),
  );
}

export function parseAuth(json: string): Auth {
  try {
    const v = JSON.parse(json);
    return {
      type_: v.type_ ?? "none",
      token: v.token ?? "",
      username: v.username ?? "",
      password: v.password ?? "",
      key: v.key ?? "",
      value: v.value ?? "",
      in_header: v.in_header ?? true,
    };
  } catch {
    return { type_: "none", token: "", username: "", password: "", key: "", value: "", in_header: true };
  }
}

export function authToJson(auth: Auth): string {
  return JSON.stringify(auth);
}

// ---------- file save / pick (Tauri dialog + native fs commands) ----------

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function browserDownload(name: string, content: string): Promise<void> {
  const blob = new Blob([content], { type: "text/plain" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
  URL.revokeObjectURL(url);
}

export async function saveFile(defaultName: string, content: string): Promise<void> {
  if (!isTauri()) return browserDownload(defaultName, content);
  const { save } = await import("@tauri-apps/plugin-dialog");
  const path = await save({ defaultPath: defaultName });
  if (typeof path === "string") {
    await invoke("save_file", { path, content });
  }
}

export async function pickFile(): Promise<string | null> {
  if (!isTauri()) {
    return new Promise((resolve) => {
      const input = document.createElement("input");
      input.type = "file";
      input.accept = ".json,application/json";
      input.onchange = () => {
        const f = input.files?.[0];
        if (!f) return resolve(null);
        const reader = new FileReader();
        reader.onload = () => resolve(String(reader.result));
        reader.onerror = () => resolve(null);
        reader.readAsText(f);
      };
      input.click();
    });
  }
  const { open } = await import("@tauri-apps/plugin-dialog");
  const res = await open({ multiple: false, filters: [{ name: "Poster project", extensions: ["json"] }] });
  if (typeof res !== "string") return null;
  return invoke<string>("read_file", { path: res });
}

export function downloadReport(report: LoadTestReport) {
  const csv = "seq,status,latency_ms,success\n" +
    report.per_request.map((r) => `${r.seq},${r.status},${r.latency_ms},${r.success}`).join("\n");
  saveFile("poster-report.csv", csv);
  saveFile("poster-report.json", JSON.stringify(report, null, 2));
}
