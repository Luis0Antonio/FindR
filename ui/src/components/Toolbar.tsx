import type { FileKind } from "../api";
import { useApp, MAX_ITEM, MIN_ITEM, type PaneId } from "../state/panes";
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

function Breadcrumb({ id, path }: { id: PaneId; path: string }) {
  const navigate = useApp((s) => s.navigate);
  const parts = path.split("/").filter(Boolean);
  const crumbs = [{ label: baseName("/"), path: "/" }, ...parts.map((p, i) => ({ label: p, path: "/" + parts.slice(0, i + 1).join("/") }))];
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
  const s = useApp.getState();
  return (
    <div className="toolbar-wrap">
      <div className="toolbar">
        <div className="nav-buttons">
          <button className="tb" title="Back (⌘[)" disabled={pane.historyIndex <= 0} onClick={() => s.back(id)}>
            ‹
          </button>
          <button className="tb" title="Forward (⌘])" disabled={pane.historyIndex >= pane.history.length - 1} onClick={() => s.forward(id)}>
            ›
          </button>
          <button className="tb" title="Enclosing Folder (⌘↑)" disabled={pane.path === "/"} onClick={() => s.up(id)}>
            ↑
          </button>
        </div>
        <Breadcrumb id={id} path={pane.path} />
        <button className="tb" title="New Folder (⇧⌘N)" onClick={() => s.createFolder(id)}>
          <svg width="18" height="14" viewBox="0 0 18 14" fill="none" stroke="currentColor" strokeWidth="1.2">
            <path d="M1.5 2.5h5l1.5 1.5h8.5v8.5h-15z" />
            <path d="M9 6v5M6.5 8.5h5" />
          </svg>
        </button>
        <label className="size-slider" title="Item size (⌘= / ⌘-)">
          <span className="small-a">A</span>
          <input type="range" min={MIN_ITEM} max={MAX_ITEM} step={1} value={pane.itemSize} onChange={(e) => s.setItemSize(id, Number(e.target.value))} />
          <span className="big-a">A</span>
        </label>
        <input
          className="search"
          data-search={id}
          type="search"
          placeholder={pane.recursive ? "Search subfolders" : "Filter"}
          value={pane.nameFilter}
          spellCheck={false}
          autoCorrect="off"
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
      </div>
      <div className="chips">
        <label className={`chip toggle${pane.recursive ? " on" : ""}`} title="Search inside subfolders">
          <input type="checkbox" checked={pane.recursive} onChange={(e) => s.setRecursive(id, e.target.checked)} />
          Include subfolders
        </label>
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
    </div>
  );
}
