import { api } from "../api";
import { useApp, type PaneId } from "../state/panes";
import { FileList } from "./FileList";

export function Pane({ id }: { id: PaneId }) {
  const pane = useApp((s) => s.panes[id]);
  return (
    <section className="pane">
      <div className="pathbar">{pane.path}</div>
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
        <FileList pane={pane} entries={pane.entries} />
      )}
      <div className="statusbar">{pane.loading ? "Loading…" : `${pane.entries.length} items`}</div>
    </section>
  );
}
