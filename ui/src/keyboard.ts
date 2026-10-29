import { activePaneState, useApp } from "./state/panes";

function inTextField(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  return !!el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.isContentEditable);
}

export function handleKeyDown(e: KeyboardEvent) {
  if (inTextField(e.target) || useApp.getState().goToOpen) return;
  const s = useApp.getState();
  const { id, pane } = activePaneState();
  const cmd = e.metaKey;
  let handled = true;

  if (cmd && e.shiftKey && e.code === "KeyG") s.setGoToOpen(true);
  else if (cmd && e.key === "ArrowUp") s.up(id);
  else if (cmd && e.key === "ArrowDown") s.openSelection(id);
  else if (cmd && e.code === "BracketLeft") s.back(id);
  else if (cmd && e.code === "BracketRight") s.forward(id);
  else if (cmd && (e.key === "=" || e.key === "+")) s.setItemSize(id, pane.itemSize + 4);
  else if (cmd && e.key === "-") s.setItemSize(id, pane.itemSize - 4);
  else if (cmd && !e.shiftKey && e.code === "KeyA") s.selectAll(id);
  else if (!cmd && e.key === "ArrowUp") s.moveCursor(id, -1, e.shiftKey);
  else if (!cmd && e.key === "ArrowDown") s.moveCursor(id, 1, e.shiftKey);
  else if (e.key === "Escape") s.clearSelection(id);
  else handled = false;

  if (handled) e.preventDefault();
}
