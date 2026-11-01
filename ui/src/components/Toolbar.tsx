import { useState } from "react";
import type { FileKind } from "../api";
import { useApp, type PaneId } from "../state/panes";
import { baseName } from "../format";

const CHIPS: { kind: FileKind; label: string }[] = [
  { kind: "folder", label: "Folders" },
  { kind: "image", label: "Images" },
  { kind: "document", label: "Documents" },
  { kind: "video", label: "Video" },
  { kind: "audio", label: "Audio" },
  { kind: "archive", label: "Archives" },
  { kind: "code", label: "Code" },
  { kind: "other", label: "Other" },
];

export function Breadcrumb({ id, path }: { id: PaneId; path: string }) {
  const navigate = useApp((s) => s.navigate);
  const parts = path.split("/").filter(Boolean);
  const crumbs = [
    { label: baseName("/"), path: "/" },
    ...parts.map((p, i) => ({
      label: p,
      path: "/" + parts.slice(0, i + 1).join("/"),
    })),
  ];
  return (
    <div className="breadcrumb" title={path}>
      {crumbs.map((c, i) => (
        <span key={c.path} className="crumb-wrap">
          {i > 0 && <span className="crumb-sep">›</span>}
          <button className="crumb" onClick={() => navigate(id, c.path)}>
            {c.label}
          </button>
        </span>
      ))}
    </div>
  );
}

export function focusSearch(id: PaneId) {
  const input = document.querySelector<HTMLInputElement>(`input[data-search="${id}"]`);
  input?.focus();
  input?.select();
}

export function Toolbar({ id }: { id: PaneId }) {
  const pane = useApp((s) => s.panes[id]);
  const split = useApp((s) => s.splitEnabled);
  const [focused, setFocused] = useState(false);
  const s = useApp.getState();
  const searching = focused || pane.nameFilter !== "" || pane.kindFilter.size > 0;
  return (
    <div className="toolbar-wrap">
      <div className="toolbar" data-tauri-drag-region>
        <div className="capsule">
          <button className="tb" title="Back (⌘[)" disabled={pane.historyIndex <= 0} onClick={() => s.back(id)}>
            <svg width="9" height="14" viewBox="0 0 9 14" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
              <path d="M7 1.5 1.5 7 7 12.5" />
            </svg>
          </button>
          <button className="tb" title="Forward (⌘])" disabled={pane.historyIndex >= pane.history.length - 1} onClick={() => s.forward(id)}>
            <svg width="9" height="14" viewBox="0 0 9 14" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
              <path d="M2 1.5 7.5 7 2 12.5" />
            </svg>
          </button>
        </div>
        <div className="title" data-tauri-drag-region title={pane.path}>
          {baseName(pane.path)}
        </div>
        <div className="capsule">
          <button className="tb" title="New Folder (⇧⌘N)" onClick={() => s.createFolder(id)}>
            <svg width="18" height="14" viewBox="0 0 18 14" fill="none" stroke="currentColor" strokeWidth="1.2">
              <path d="M1.5 2.5h5l1.5 1.5h8.5v8.5h-15z" />
              <path d="M9 6v5M6.5 8.5h5" />
            </svg>
          </button>
          {id === "left" && (
            <button className={`tb${split ? " on" : ""}`} title="Split View (⌘\)" onClick={() => s.toggleSplit()}>
              <svg width="18" height="14" viewBox="0 0 18 14" fill="none" stroke="currentColor" strokeWidth="1.2">
                <rect x="1.5" y="1.5" width="15" height="11" rx="1.5" />
                <path d="M9 1.5v11" />
              </svg>
            </button>
          )}
        </div>
        <label className="search-field">
          <svg width="13" height="13" viewBox="0 0 13 13" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
            <circle cx="5.5" cy="5.5" r="4.25" />
            <path d="m8.7 8.7 3.3 3.3" />
          </svg>
          <input
            className="search"
            data-search={id}
            type="search"
            placeholder="Search"
            value={pane.nameFilter}
            spellCheck={false}
            autoCorrect="off"
            onFocus={() => setFocused(true)}
            onBlur={() => setFocused(false)}
            onChange={(e) => s.setNameFilter(id, e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                e.preventDefault();
                s.setNameFilter(id, "");
                if (pane.recursive) s.runSearch(id);
                e.currentTarget.blur();
              } else if (e.key === "Enter" || e.key === "ArrowDown") {
                e.preventDefault();
                e.currentTarget.blur();
                if (e.key === "ArrowDown") s.moveCursor(id, 1, false);
              }
            }}
          />
        </label>
      </div>
      {searching && (
        // Keeps focus in the search field, so clicking a filter doesn't hide the bar mid-click.
        <div className="scope" onMouseDown={(e) => e.preventDefault()}>
          <span className="scope-label">Search:</span>
          <button className={`chip${pane.recursive ? "" : " on"}`} onClick={() => s.setRecursive(id, false)}>
            This folder
          </button>
          <button className={`chip${pane.recursive ? " on" : ""}`} onClick={() => s.setRecursive(id, true)}>
            Subfolders
          </button>
          <span className="chip-sep" />
          {CHIPS.map((c) => (
            <button key={c.kind} className={`chip${pane.kindFilter.has(c.kind) ? " on" : ""}`} onClick={() => s.toggleKind(id, c.kind)}>
              {c.label}
            </button>
          ))}
          {pane.kindFilter.size > 0 && (
            <button className="chip clear" onClick={() => s.clearKinds(id)}>
              Clear
            </button>
          )}
        </div>
      )}
    </div>
  );
}
