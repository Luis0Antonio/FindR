import { api } from "./api";
import { parentOf } from "./format";
import { useApp, type PaneId } from "./state/panes";
import { visibleEntries } from "./state/visible";
import { transfer } from "./transfer";

export async function copySelection(id: PaneId, cut: boolean) {
  const pane = useApp.getState().panes[id];
  const paths = visibleEntries(pane)
    .filter((e) => pane.selection.has(e.path))
    .map((e) => e.path);
  if (!paths.length) return;
  const changeCount = await api.clipboardWriteFiles(paths);
  useApp.setState({ clipboard: { paths, cut, changeCount } });
}

/** Pastes files from the system clipboard into the pane's folder; files cut in FindR are moved, anything else is copied. */
export async function paste(id: PaneId) {
  const target = useApp.getState().panes[id].path;
  const { paths, changeCount } = await api.clipboardReadFiles();
  if (!paths.length) return;
  const clip = useApp.getState().clipboard;
  const cut = clip?.cut === true && clip.changeCount === changeCount;
  if (cut) useApp.setState({ clipboard: null });
  // Copying next to the original makes a numbered duplicate instead of asking to replace it.
  const duplicate = !cut && paths.every((p) => parentOf(p) === target);
  await transfer(paths, target, !cut, duplicate ? "keepBoth" : null);
}
