export interface Project {
  id: number;
  name: string;
  created_at: string;
  updated_at: string;
}

export interface Request {
  id: number;
  project_id: number;
  name: string;
  method: string;
  url: string;
  headers_json: string;
  body_type: string;
  body: string;
  auth_json: string;
}

export interface Variable {
  id: number;
  project_id: number;
  key: string;
  value: string;
}

export interface Extract {
  id: number;
  request_id: number;
  source_request_id: number | null;
  jsonpath: string;
  target_var_key: string;
}

export interface Header {
  key: string;
  value: string;
}

export type AuthType = "none" | "bearer" | "basic" | "api_key";

export interface Auth {
  type_: AuthType;
  token: string;
  username: string;
  password: string;
  key: string;
  value: string;
  in_header: boolean;
}

export interface VarOverride {
  key: string;
  mode: "fixed" | "range" | "list";
  value: string;
  value_end?: string | null;
}

export interface LoadTestConfig {
  url: string;
  method: string;
  headers: Header[];
  body_type: string;
  body: string;
  auth: Auth;
  variables: [string, string][];
  overrides: VarOverride[];
  rps: number;
  total_requests: number;
  concurrency: number;
  timeout_ms: number;
}

export interface ResponseData {
  status: number;
  status_text: string;
  time_ms: number;
  size_bytes: number;
  headers: [string, string][];
  body: string;
}

export interface CurlImportResult {
  method: string;
  url: string;
  headers: Header[];
  body_type: string;
  body: string;
  auth: Auth;
  warnings: string[];
}

export interface LoadTestStats {
  completed: number;
  successes: number;
  failures: number;
  avg_ms: number;
  p95_ms: number;
  elapsed_ms: number;
}

export interface PerRequest {
  seq: number;
  status: number;
  latency_ms: number;
  success: boolean;
}

export interface LoadTestReport {
  total: number;
  successes: number;
  failures: number;
  duration_ms: number;
  avg_ms: number;
  p95_ms: number;
  rps_actual: number;
  per_request: PerRequest[];
}

export const defaultAuth = (): Auth => ({
  type_: "none",
  token: "",
  username: "",
  password: "",
  key: "",
  value: "",
  in_header: true,
});
