import { api, toFsError } from "../api";
import { useApp } from "../state/panes";

export function EmptyTrashDialog() {
  const open = useApp((s) => s.confirmEmptyTrash);
  if (!open) return null;
  const close = () => useApp.setState({ confirmEmptyTrash: false });
  const empty = async () => {
    close();
    useApp.setState({ activity: "Emptying the Trash…" });
    try {
      await api.emptyTrash();
    } catch (e) {
      useApp.setState({ notice: `Couldn't empty the Trash: ${toFsError(e).message}` });
    }
    useApp.setState({ activity: null });
  };
  return (
    <div className="modal-backdrop">
      <div className="modal">
        <div className="modal-title">Are you sure you want to permanently erase the items in the Trash?</div>
        <p className="modal-text">You can’t undo this action.</p>
        <div className="modal-buttons">
          <button autoFocus onClick={close}>
            Cancel
          </button>
          <button className="primary danger" onClick={empty}>
            Empty Trash
          </button>
        </div>
      </div>
    </div>
  );
}
