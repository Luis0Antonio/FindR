import { activePaneState, useApp, type PaneId } from "./state/panes";
import { focusSearch } from "./components/Toolbar";
import { closeQuickLook, isQuickLookOpen, toggleQuickLook } from "./quicklook";

function inTextField(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  return !!el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.isContentEditable);
}

function typeIntoSearch(id: PaneId, text: string) {
  useApp.getState().setNameFilter(id, text);
  const input = document.querySelector<HTMLInputElement>(`input[data-search="${id}"]`);
  input?.focus();
  requestAnimationFrame(() => input?.setSelectionRange(text.length, text.length));
}

export function handleKeyDown(e: KeyboardEvent) {
  if (useApp.getState().settingsOpen) {
    if (e.key === "Escape") useApp.getState().setSettingsOpen(false);
    return;
  }
  if (e.metaKey && !e.shiftKey && e.code === "KeyB") {
    e.preventDefault();
    useApp.getState().toggleSidebar();
    return;
  }
  if (inTextField(e.target) || useApp.getState().goToOpen || useApp.getState().conflictPrompt) return;
  const s = useApp.getState();
  const { id, pane } = activePaneState();
  const cmd = e.metaKey;
  let handled = true;

  if (cmd && e.shiftKey && e.code === "KeyS") s.toggleSplit();
  else if (cmd && e.code === "Backslash") s.openRightPane();
  else if (cmd && e.code === "KeyW") s.closePane(id);
  else if (cmd && e.key === "Backspace") s.trashSelection(id);
  else if (e.key === "Tab" && !cmd) {
    if (s.splitEnabled) s.setActive(id === "left" ? "right" : "left");
  } else if (cmd && e.shiftKey && e.code === "KeyG") s.setGoToOpen(true);
  else if (cmd && e.shiftKey && e.code === "KeyN") s.createFolder(id);
  else if (cmd && e.ctrlKey && e.code === "KeyT") {
    const sel = pane.selection.size === 1 ? [...pane.selection][0] : null;
    const entry = sel ? pane.entries.find((x) => x.path === sel) ?? pane.searchResults?.find((x) => x.path === sel) : null;
    s.addFavorite(entry?.isDir ? entry.path : pane.path);
  } else if (!cmd && e.key === "Enter" && pane.selection.size === 1) s.startRename(id);
  else if (cmd && e.shiftKey && e.code === "Period") s.toggleHidden();
  else if (cmd && e.code === "KeyF") focusSearch(id);
  else if (cmd && e.key === "ArrowUp") s.up(id);
  else if (cmd && e.key === "ArrowDown") s.openSelection(id);
  else if (cmd && e.code === "BracketLeft") s.back(id);
  else if (cmd && e.code === "BracketRight") s.forward(id);
  else if (cmd && (e.key === "=" || e.key === "+")) s.setItemSize(id, pane.itemSize + 4);
  else if (cmd && e.key === "-") s.setItemSize(id, pane.itemSize - 4);
  else if (cmd && !e.shiftKey && e.code === "KeyA") s.selectAll(id);
  else if (!cmd && e.key === "ArrowUp") s.moveCursor(id, -1, e.shiftKey);
  else if (!cmd && e.key === "ArrowDown") s.moveCursor(id, 1, e.shiftKey);
  else if (!cmd && e.key === " ") toggleQuickLook();
  else if (e.key === "Escape" && isQuickLookOpen()) closeQuickLook();
  else if (e.key === "Escape") s.clearSelection(id);
  else if (!cmd && !e.ctrlKey && !e.altKey && e.key.length === 1) typeIntoSearch(id, pane.nameFilter + e.key);
  else handled = false;

  if (handled) e.preventDefault();
}
