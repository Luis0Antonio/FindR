import { ALL_COLUMNS, useApp } from "../state/panes";
import { COLUMN_INFO } from "./FileList";

export function Settings() {
  const columns = useApp((s) => s.columns);
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
        <div className="modal-buttons">
          <button className="primary" onClick={close}>
            Done
          </button>
        </div>
      </div>
    </div>
  );
}
