import { useState } from "react";
import type { Project, Request } from "../types";

interface Props {
  projects: Project[];
  requests: Request[];
  activeProjectId: number | null;
  activeRequestId: number | null;
  onSelectProject: (id: number) => void;
  onSelectRequest: (id: number) => void;
  onCreateProject: () => void;
  onRenameProject: (id: number, name: string) => void;
  onDeleteProject: (id: number) => void;
  onCreateRequest: () => void;
  onRenameRequest: (id: number, name: string) => void;
  onDeleteRequest: (id: number) => void;
}

export default function ProjectTree({
  projects,
  requests,
  activeProjectId,
  activeRequestId,
  onSelectProject,
  onSelectRequest,
  onCreateProject,
  onRenameProject,
  onDeleteProject,
  onCreateRequest,
  onRenameRequest,
  onDeleteRequest,
}: Props) {
  const [editingProject, setEditingProject] = useState<number | null>(null);
  const [editingRequest, setEditingRequest] = useState<number | null>(null);
  const [draft, setDraft] = useState("");

  function startEdit(kind: "project" | "request", id: number, current: string) {
    if (kind === "project") setEditingProject(id);
    else setEditingRequest(id);
    setDraft(current);
  }

  function commit() {
    if (editingProject !== null) onRenameProject(editingProject, draft.trim() || "Untitled");
    if (editingRequest !== null) onRenameRequest(editingRequest, draft.trim() || "Request");
    setEditingProject(null);
    setEditingRequest(null);
  }

  return (
    <div className="left-panel">
      <div className="panel-header">
        <span>Projects</span>
        <button className="icon-btn" title="New project" onClick={onCreateProject}>+</button>
      </div>
      {projects.length === 0 && (
        <div className="empty-state" style={{ padding: 20 }}>No projects yet. Click + to create one.</div>
      )}
      {projects.map((p) => (
        <div key={p.id}>
          <div
            className={`project-item ${activeProjectId === p.id ? "active" : ""}`}
            onClick={() => onSelectProject(p.id)}
          >
            {editingProject === p.id ? (
              <input
                autoFocus
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
                onBlur={commit}
                onKeyDown={(e) => e.key === "Enter" && commit()}
                onClick={(e) => e.stopPropagation()}
              />
            ) : (
              <>
                <span className="name">{p.name}</span>
                <button
                  className="icon-btn"
                  title="Rename project"
                  onClick={(e) => {
                    e.stopPropagation();
                    startEdit("project", p.id, p.name);
                  }}
                >
                  ✎
                </button>
                <button
                  className="icon-btn"
                  title="Delete project"
                  onClick={(e) => {
                    e.stopPropagation();
                    if (confirm(`Delete project "${p.name}" and all its requests?`)) onDeleteProject(p.id);
                  }}
                >
                  ✕
                </button>
              </>
            )}
          </div>
          {activeProjectId === p.id && (
            <div style={{ paddingLeft: 14 }}>
              {requests.map((r) => (
                <div
                  key={r.id}
                  className={`request-item ${activeRequestId === r.id ? "active" : ""}`}
                  onClick={() => onSelectRequest(r.id)}
                >
                  <span className={`method-badge method-${r.method}`}>{r.method}</span>
                  {editingRequest === r.id ? (
                    <input
                      autoFocus
                      value={draft}
                      onChange={(e) => setDraft(e.target.value)}
                      onBlur={commit}
                      onKeyDown={(e) => e.key === "Enter" && commit()}
                      onClick={(e) => e.stopPropagation()}
                    />
                  ) : (
                    <>
                      <span className="name">{r.name}</span>
                      <button
                        className="icon-btn"
                        title="Rename request"
                        onClick={(e) => {
                          e.stopPropagation();
                          startEdit("request", r.id, r.name);
                        }}
                      >
                        ✎
                      </button>
                      <button
                        className="icon-btn"
                        title="Delete request"
                        onClick={(e) => {
                          e.stopPropagation();
                          if (confirm(`Delete request "${r.name}"?`)) onDeleteRequest(r.id);
                        }}
                      >
                        ✕
                      </button>
                    </>
                  )}
                </div>
              ))}
              <button
                className="icon-btn"
                style={{ margin: "4px 0 8px 14px", color: "#6c8cff" }}
                onClick={onCreateRequest}
              >
                + New request
              </button>
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
