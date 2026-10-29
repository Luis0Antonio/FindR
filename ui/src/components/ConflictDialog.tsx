import { useState } from "react";
import type { ConflictPolicy } from "../api";
import { useApp } from "../state/panes";

export function ConflictDialog() {
  const prompt = useApp((s) => s.conflictPrompt);
  const [all, setAll] = useState(false);
  if (!prompt) return null;
  const choose = (policy: ConflictPolicy) => prompt.resolve({ policy, all });
  return (
    <div className="modal-backdrop">
      <div className="modal conflict">
        <div className="modal-title">An item named “{prompt.name}” already exists in this location.</div>
        <p className="modal-text">Do you want to replace it with the one you’re moving? Replaced items go to the Trash.</p>
        {prompt.remaining > 0 && (
          <label className="apply-all">
            <input type="checkbox" checked={all} onChange={(e) => setAll(e.target.checked)} /> Apply to all ({prompt.remaining + 1} conflicts)
          </label>
        )}
        <div className="modal-buttons">
          <button onClick={() => choose("skip")}>Skip</button>
          <button onClick={() => choose("keepBoth")}>Keep Both</button>
          <button className="primary" autoFocus onClick={() => choose("replace")}>
            Replace
          </button>
        </div>
      </div>
    </div>
  );
}
