import { useState } from "react";
import { useApp } from "../state/panes";

export function GoToFolder() {
  const [value, setValue] = useState("");
  const close = () => useApp.getState().setGoToOpen(false);
  const go = () => {
    const s = useApp.getState();
    let path = value.trim();
    if (!path) return;
    if (path === "~" || path.startsWith("~/")) path = s.home + path.slice(1);
    if (!path.startsWith("/")) path = `${s.panes[s.activePane].path}/${path}`;
    path = path.replace(/\/+$/, "") || "/";
    close();
    s.navigate(s.activePane, path);
  };
  return (
    <div className="modal-backdrop" onPointerDown={close}>
      <div className="modal" onPointerDown={(e) => e.stopPropagation()}>
        <div className="modal-title">Go to the folder:</div>
        <input
          autoFocus
          value={value}
          placeholder="/path/to/folder or ~/folder"
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") go();
            if (e.key === "Escape") close();
          }}
        />
        <div className="modal-buttons">
          <button onClick={close}>Cancel</button>
          <button className="primary" onClick={go}>
            Go
          </button>
        </div>
      </div>
    </div>
  );
}
