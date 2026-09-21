import { useState } from "react";
import type { Variable } from "../types";

interface Props {
  variables: Variable[];
  onUpsert: (key: string, value: string) => void;
  onDelete: (id: number) => void;
}

export default function VariablesPanel({ variables, onUpsert, onDelete }: Props) {
  const [newKey, setNewKey] = useState("");
  const [newValue, setNewValue] = useState("");

  function addVar() {
    if (!newKey.trim()) return;
    onUpsert(newKey.trim(), newValue);
    setNewKey("");
    setNewValue("");
  }

  return (
    <div className="var-list">
      <p className="hint">Project variables. Use {"{{key}}"} in URLs, headers, body and auth.</p>
      {variables.map((v) => (
        <div key={v.id} className="var-row">
          <input value={v.key} readOnly />
          <input
            defaultValue={v.value}
            onBlur={(e) => onUpsert(v.key, e.target.value)}
          />
          <button className="icon-btn" onClick={() => onDelete(v.id)}>✕</button>
        </div>
      ))}
      <div className="var-add">
        <input
          value={newKey}
          onChange={(e) => setNewKey(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && addVar()}
          placeholder="key"
        />
        <input
          value={newValue}
          onChange={(e) => setNewValue(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && addVar()}
          placeholder="value"
        />
        <button className="add-var-btn" disabled={!newKey.trim()} onClick={addVar}>
          Add variable
        </button>
      </div>
    </div>
  );
}
