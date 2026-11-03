import { useEffect, useState } from "react";
import { api } from "../api";
import { ALL_COLUMNS, useApp } from "../state/panes";
import { COLUMN_INFO } from "./FileList";

function SearchSettings() {
  const roots = useApp((s) => s.indexRoots);
  const excluded = useApp((s) => s.excludedFolders);
  const current = useApp((s) => s.panes[s.activePane].path);
  const home = useApp((s) => s.home);
  const [status, setStatus] = useState<{ building: boolean; items: number } | null>(null);
  const [name, setName] = useState("");
  const s = useApp.getState();

  useEffect(() => {
    const poll = () => api.indexStatus().then(setStatus);
    poll();
    const timer = window.setInterval(poll, 1000);
    return () => window.clearInterval(timer);
  }, []);

  const covered = roots.some((r) => current === r || current.startsWith(`${r}/`));
  const short = (path: string) => (path.startsWith(`${home}/`) ? `~${path.slice(home.length)}` : path);
  const addName = () => {
    const n = name.trim();
    if (n && !excluded.includes(n)) s.setExcludedFolders([...excluded, n]);
    setName("");
  };

  return (
    <section className="settings-section">
      <div className="settings-heading">Search</div>
      <p className="modal-text">FindR keeps a list of the names in these folders, so searching them with Subfolders is instant.</p>
      <div className="index-roots">
        {roots.map((r) => (
          <div key={r} className="index-root">
            <span title={r}>{short(r)}</span>
            <button className="chip-remove" title="Stop indexing this folder" onClick={() => s.setIndexRoots(roots.filter((x) => x !== r))}>
              ×
            </button>
          </div>
        ))}
      </div>
      <div className="index-actions">
        <button disabled={covered} onClick={() => s.setIndexRoots([...roots, current])}>
          Add Current Folder
        </button>
        <span className="modal-text">{status?.building ? "Indexing…" : status ? `${status.items.toLocaleString()} items indexed` : ""}</span>
      </div>
      <div className="settings-heading">Skip folders named</div>
      <div className="excluded">
        {excluded.map((n) => (
          <span key={n} className="excluded-name">
            {n}
            <button className="chip-remove" title="Search inside these folders" onClick={() => s.setExcludedFolders(excluded.filter((x) => x !== n))}>
              ×
            </button>
          </span>
        ))}
        <input placeholder="Add a name" value={name} onChange={(e) => setName(e.target.value)} onKeyDown={(e) => e.key === "Enter" && addName()} />
      </div>
    </section>
  );
}

export function Settings() {
  const columns = useApp((s) => s.columns);
  const keepOriginals = useApp((s) => s.keepConvertedOriginals);
  const s = useApp.getState();
  const close = () => s.setSettingsOpen(false);
  return (
    <div className="modal-backdrop" onPointerDown={close}>
      <div className="modal settings" onPointerDown={(e) => e.stopPropagation()}>
        <div className="modal-title">Settings</div>
        <section className="settings-section">
          <div className="settings-heading">Columns</div>
          <p className="modal-text">Choose which columns the file list shows.</p>
          <label className="settings-option disabled">
            <input type="checkbox" checked disabled />
            Name
          </label>
          {ALL_COLUMNS.map((c) => (
            <label key={c} className="settings-option">
              <input type="checkbox" checked={columns.includes(c)} onChange={() => s.toggleColumn(c)} />
              {COLUMN_INFO[c].label}
            </label>
          ))}
        </section>
        <section className="settings-section">
          <div className="settings-heading">Converting files</div>
          <p className="modal-text">Choose what happens to the original file after a conversion.</p>
          <label className="settings-option">
            <input type="radio" name="convert-originals" checked={!keepOriginals} onChange={() => s.setKeepConvertedOriginals(false)} />
            Replace it (the original goes to the Trash)
          </label>
          <label className="settings-option">
            <input type="radio" name="convert-originals" checked={keepOriginals} onChange={() => s.setKeepConvertedOriginals(true)} />
            Keep it and create a new file
          </label>
        </section>
        <SearchSettings />
        <div className="modal-buttons">
          <button className="primary" onClick={close}>
            Done
          </button>
        </div>
      </div>
    </div>
  );
}
