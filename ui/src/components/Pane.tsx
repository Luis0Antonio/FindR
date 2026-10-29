import { useMemo } from "react";
import { api } from "../api";
import { useApp, type PaneId } from "../state/panes";
import { visibleEntries } from "../state/visible";
import { FileList } from "./FileList";
import { Toolbar } from "./Toolbar";

export function Pane({ id }: { id: PaneId }) {
  const pane = useApp((s) => s.panes[id]);
  const active = useApp((s) => s.activePane === id);
  const split = useApp((s) => s.splitEnabled);
  const entries = useMemo(() => visibleEntries(pane), [pane]);
  return (
    <section className={`pane${active && split ? " active" : ""}`} onPointerDownCapture={() => useApp.getState().setActive(id)}>
      <Toolbar id={id} />
      {pane.error ? (
        <div className="pane-message">
          {pane.error.code === "permission_denied" ? (
            <>
              <p>No permission to read this folder.</p>
              <button onClick={() => api.openPrivacySettings()}>Open System Settings</button>
            </>
          ) : (
            <p>{pane.error.message}</p>
          )}
        </div>
      ) : (
        <FileList id={id} pane={pane} entries={entries} />
      )}
      <div className="statusbar">
        <span>
          {pane.selection.size > 0 ? `${pane.selection.size} of ` : ""}
          {entries.length} items
        </span>
        {pane.loading && <span>Loading…</span>}
      </div>
    </section>
  );
}
