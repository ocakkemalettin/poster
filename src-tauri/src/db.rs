use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers_json: String,
    pub body_type: String,
    pub body: String,
    pub auth_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub id: i64,
    pub project_id: i64,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extract {
    pub id: i64,
    pub request_id: i64,
    pub source_request_id: Option<i64>,
    pub jsonpath: String,
    pub target_var_key: String,
}

pub struct Db(pub Mutex<Connection>);

impl Db {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS projects (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS requests (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                method TEXT NOT NULL DEFAULT 'GET',
                url TEXT NOT NULL DEFAULT '',
                headers_json TEXT NOT NULL DEFAULT '[]',
                body_type TEXT NOT NULL DEFAULT 'none',
                body TEXT NOT NULL DEFAULT '',
                auth_json TEXT NOT NULL DEFAULT '{}'
            );
            CREATE TABLE IF NOT EXISTS variables (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                key TEXT NOT NULL,
                value TEXT NOT NULL DEFAULT ''
            );
            CREATE TABLE IF NOT EXISTS extracts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                request_id INTEGER NOT NULL REFERENCES requests(id) ON DELETE CASCADE,
                source_request_id INTEGER REFERENCES requests(id) ON DELETE SET NULL,
                jsonpath TEXT NOT NULL,
                target_var_key TEXT NOT NULL
            );",
        )?;
        Ok(Self(Mutex::new(conn)))
    }

    pub fn with_conn<F: FnOnce(&Connection) -> Result<T>, T>(&self, f: F) -> Result<T> {
        let guard = self.0.lock().unwrap();
        f(&guard)
    }
}

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

impl Db {
    pub fn create_project(&self, name: &str) -> Result<Project> {
        self.with_conn(|conn| {
            let ts = now_iso();
            conn.execute(
                "INSERT INTO projects (name, created_at, updated_at) VALUES (?1, ?2, ?3)",
                params![name, ts, ts],
            )?;
            let id = conn.last_insert_rowid();
            Ok(Project {
                id,
                name: name.to_string(),
                created_at: ts.clone(),
                updated_at: ts,
            })
        })
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT id, name, created_at, updated_at FROM projects ORDER BY id")?;
            let rows = stmt.query_map([], |r| {
                Ok(Project {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    created_at: r.get(2)?,
                    updated_at: r.get(3)?,
                })
            })?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
    }

    pub fn rename_project(&self, id: i64, name: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE projects SET name = ?1, updated_at = ?2 WHERE id = ?3",
                params![name, now_iso(), id],
            )?;
            Ok(())
        })
    }

    pub fn delete_project(&self, id: i64) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    pub fn create_request(&self, project_id: i64, name: &str) -> Result<Request> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO requests (project_id, name, method, url, headers_json, body_type, body, auth_json)
                 VALUES (?1, ?2, 'GET', '', '[]', 'none', '', '{}')",
                params![project_id, name],
            )?;
            let id = conn.last_insert_rowid();
            Ok(Request {
                id,
                project_id,
                name: name.to_string(),
                method: "GET".into(),
                url: String::new(),
                headers_json: "[]".into(),
                body_type: "none".into(),
                body: String::new(),
                auth_json: "{}".into(),
            })
        })
    }

    pub fn list_requests(&self, project_id: i64) -> Result<Vec<Request>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, project_id, name, method, url, headers_json, body_type, body, auth_json
                 FROM requests WHERE project_id = ?1 ORDER BY id",
            )?;
            let rows = stmt.query_map(params![project_id], |r| {
                Ok(Request {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    name: r.get(2)?,
                    method: r.get(3)?,
                    url: r.get(4)?,
                    headers_json: r.get(5)?,
                    body_type: r.get(6)?,
                    body: r.get(7)?,
                    auth_json: r.get(8)?,
                })
            })?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
    }

    pub fn update_request(&self, req: &Request) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE requests SET name = ?1, method = ?2, url = ?3, headers_json = ?4, body_type = ?5, body = ?6, auth_json = ?7 WHERE id = ?8",
                params![req.name, req.method, req.url, req.headers_json, req.body_type, req.body, req.auth_json, req.id],
            )?;
            Ok(())
        })
    }

    pub fn get_request(&self, id: i64) -> Result<Option<Request>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, project_id, name, method, url, headers_json, body_type, body, auth_json FROM requests WHERE id = ?1",
            )?;
            let mut rows = stmt.query_map(params![id], |r| {
                Ok(Request {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    name: r.get(2)?,
                    method: r.get(3)?,
                    url: r.get(4)?,
                    headers_json: r.get(5)?,
                    body_type: r.get(6)?,
                    body: r.get(7)?,
                    auth_json: r.get(8)?,
                })
            })?;
            match rows.next() {
                Some(Ok(r)) => Ok(Some(r)),
                Some(Err(e)) => Err(e),
                None => Ok(None),
            }
        })
    }

    pub fn delete_request(&self, id: i64) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM requests WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    pub fn list_variables(&self, project_id: i64) -> Result<Vec<Variable>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, project_id, key, value FROM variables WHERE project_id = ?1 ORDER BY id",
            )?;
            let rows = stmt.query_map(params![project_id], |r| {
                Ok(Variable {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    key: r.get(2)?,
                    value: r.get(3)?,
                })
            })?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
    }

    pub fn upsert_variable(&self, project_id: i64, key: &str, value: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO variables (project_id, key, value) VALUES (?1, ?2, ?3)
                 ON CONFLICT DO NOTHING",
                params![project_id, key, value],
            )?;
            let n = conn.execute(
                "UPDATE variables SET value = ?1 WHERE project_id = ?2 AND key = ?3",
                params![value, project_id, key],
            )?;
            if n == 0 {
                conn.execute(
                    "INSERT INTO variables (project_id, key, value) VALUES (?1, ?2, ?3)",
                    params![project_id, key, value],
                )?;
            }
            Ok(())
        })
    }

    pub fn delete_variable(&self, id: i64) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM variables WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    pub fn list_extracts(&self, request_id: i64) -> Result<Vec<Extract>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, request_id, source_request_id, jsonpath, target_var_key FROM extracts WHERE request_id = ?1 ORDER BY id",
            )?;
            let rows = stmt.query_map(params![request_id], |r| {
                Ok(Extract {
                    id: r.get(0)?,
                    request_id: r.get(1)?,
                    source_request_id: r.get(2)?,
                    jsonpath: r.get(3)?,
                    target_var_key: r.get(4)?,
                })
            })?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
    }

    pub fn create_extract(&self, e: &Extract) -> Result<Extract> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO extracts (request_id, source_request_id, jsonpath, target_var_key) VALUES (?1, ?2, ?3, ?4)",
                params![e.request_id, e.source_request_id, e.jsonpath, e.target_var_key],
            )?;
            let id = conn.last_insert_rowid();
            Ok(Extract {
                id,
                request_id: e.request_id,
                source_request_id: e.source_request_id,
                jsonpath: e.jsonpath.clone(),
                target_var_key: e.target_var_key.clone(),
            })
        })
    }

    pub fn delete_extract(&self, id: i64) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM extracts WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    pub fn import_project(
        &self,
        name: &str,
        variables: &[(String, String)],
        requests: &[serde_json::Value],
    ) -> Result<Project> {
        let conn = self.0.lock().unwrap();
        let ts = now_iso();
        conn.execute(
            "INSERT INTO projects (name, created_at, updated_at) VALUES (?1, ?2, ?3)",
            params![name, ts, ts],
        )?;
        let pid = conn.last_insert_rowid();

        for (k, v) in variables {
            conn.execute(
                "INSERT INTO variables (project_id, key, value) VALUES (?1, ?2, ?3)",
                params![pid, k, v],
            )?;
        }

        // First pass: create requests, map old index -> new id
        let mut new_ids: Vec<i64> = Vec::new();
        for r in requests {
            conn.execute(
                "INSERT INTO requests (project_id, name, method, url, headers_json, body_type, body, auth_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    pid,
                    r.get("name").and_then(|v| v.as_str()).unwrap_or("Request"),
                    r.get("method").and_then(|v| v.as_str()).unwrap_or("GET"),
                    r.get("url").and_then(|v| v.as_str()).unwrap_or(""),
                    r.get("headers").map(|h| serde_json::to_string(h).unwrap_or_else(|_| "[]".into())).unwrap_or_else(|| "[]".into()),
                    r.get("body_type").and_then(|v| v.as_str()).unwrap_or("none"),
                    r.get("body").and_then(|v| v.as_str()).unwrap_or(""),
                    r.get("auth").map(|a| serde_json::to_string(a).unwrap_or_else(|_| "{}".into())).unwrap_or_else(|| "{}".into()),
                ],
            )?;
            new_ids.push(conn.last_insert_rowid());
        }

        // Second pass: extracts (source_request_id is index into requests array or null)
        for (i, r) in requests.iter().enumerate() {
            if let Some(exs) = r.get("extracts").and_then(|v| v.as_array()) {
                for e in exs {
                    let src_idx = e
                        .get("source_request_id")
                        .and_then(|v| v.as_u64());
                    let src: Option<i64> = src_idx.and_then(|idx| new_ids.get(idx as usize).copied());
                    conn.execute(
                        "INSERT INTO extracts (request_id, source_request_id, jsonpath, target_var_key) VALUES (?1, ?2, ?3, ?4)",
                        params![
                            new_ids[i],
                            src,
                            e.get("jsonpath").and_then(|v| v.as_str()).unwrap_or(""),
                            e.get("target_var_key").and_then(|v| v.as_str()).unwrap_or(""),
                        ],
                    )?;
                }
            }
        }

        Ok(Project {
            id: pid,
            name: name.to_string(),
            created_at: ts.clone(),
            updated_at: ts,
        })
    }

    pub fn append_requests_to_project(
        &self,
        project_id: i64,
        variables: &[(String, String)],
        requests: &[serde_json::Value],
    ) -> Result<()> {
        let conn = self.0.lock().unwrap();
        for (k, v) in variables {
            conn.execute(
                "INSERT INTO variables (project_id, key, value) VALUES (?1, ?2, ?3)",
                params![project_id, k, v],
            )?;
        }
        let mut new_ids: Vec<i64> = Vec::new();
        for r in requests {
            conn.execute(
                "INSERT INTO requests (project_id, name, method, url, headers_json, body_type, body, auth_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    project_id,
                    r.get("name").and_then(|v| v.as_str()).unwrap_or("Request"),
                    r.get("method").and_then(|v| v.as_str()).unwrap_or("GET"),
                    r.get("url").and_then(|v| v.as_str()).unwrap_or(""),
                    r.get("headers").map(|h| serde_json::to_string(h).unwrap_or_else(|_| "[]".into())).unwrap_or_else(|| "[]".into()),
                    r.get("body_type").and_then(|v| v.as_str()).unwrap_or("none"),
                    r.get("body").and_then(|v| v.as_str()).unwrap_or(""),
                    r.get("auth").map(|a| serde_json::to_string(a).unwrap_or_else(|_| "{}".into())).unwrap_or_else(|| "{}".into()),
                ],
            )?;
            new_ids.push(conn.last_insert_rowid());
        }
        for (i, r) in requests.iter().enumerate() {
            if let Some(exs) = r.get("extracts").and_then(|v| v.as_array()) {
                for e in exs {
                    let src_idx = e.get("source_request_id").and_then(|v| v.as_u64());
                    let src: Option<i64> = src_idx.and_then(|idx| new_ids.get(idx as usize).copied());
                    conn.execute(
                        "INSERT INTO extracts (request_id, source_request_id, jsonpath, target_var_key) VALUES (?1, ?2, ?3, ?4)",
                        params![
                            new_ids[i],
                            src,
                            e.get("jsonpath").and_then(|v| v.as_str()).unwrap_or(""),
                            e.get("target_var_key").and_then(|v| v.as_str()).unwrap_or(""),
                        ],
                    )?;
                }
            }
        }
        conn.execute(
            "UPDATE projects SET updated_at = ?1 WHERE id = ?2",
            params![now_iso(), project_id],
        )?;
        Ok(())
    }
}
