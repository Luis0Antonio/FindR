import { useApp, MAX_ITEM, MIN_ITEM, type PaneId } from "../state/panes";
import { baseName } from "../format";

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

export function Toolbar({ id }: { id: PaneId }) {
  const pane = useApp((s) => s.panes[id]);
  const { back, forward, up, setItemSize } = useApp.getState();
  return (
    <div className="toolbar">
      <div className="nav-buttons">
        <button className="tb" title="Back (⌘[)" disabled={pane.historyIndex <= 0} onClick={() => back(id)}>
          ‹
        </button>
        <button className="tb" title="Forward (⌘])" disabled={pane.historyIndex >= pane.history.length - 1} onClick={() => forward(id)}>
          ›
        </button>
        <button className="tb" title="Enclosing Folder (⌘↑)" disabled={pane.path === "/"} onClick={() => up(id)}>
          ↑
        </button>
      </div>
      <Breadcrumb id={id} path={pane.path} />
      <label className="size-slider" title="Item size (⌘= / ⌘-)">
        <span className="small-a">A</span>
        <input
          type="range"
          min={MIN_ITEM}
          max={MAX_ITEM}
          step={1}
          value={pane.itemSize}
          onChange={(e) => setItemSize(id, Number(e.target.value))}
        />
        <span className="big-a">A</span>
      </label>
    </div>
  );
}
