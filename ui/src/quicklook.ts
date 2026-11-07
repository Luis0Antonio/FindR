import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useApp } from "./state/panes";
import { visibleEntries } from "./state/visible";

let open = false;
let lastSent = "";

function currentItems(): { paths: string[]; index: number } | null {
  const s = useApp.getState();
  const pane = s.panes[s.activePane];
  const paths = visibleEntries(pane)
    .filter((e) => pane.selection.has(e.path))
    .map((e) => e.path);
  if (paths.length === 0) return null;
  return { paths, index: Math.max(0, paths.indexOf(pane.cursor ?? "")) };
}

export function isQuickLookOpen() {
  return open;
}

export function closeQuickLook() {
  if (!open) return;
  open = false;
  invoke("quicklook_close");
}

export function toggleQuickLook() {
  if (open) return closeQuickLook();
  const items = currentItems();
  if (!items) return;
  open = true;
  lastSent = JSON.stringify(items);
  invoke("quicklook_show", items);
}

export function startQuickLookSync() {
  useApp.subscribe((s, prev) => {
    if (!open) return;
    const pane = s.panes[s.activePane];
    const before = prev.panes[prev.activePane];
    if (s.activePane === prev.activePane && pane.selection === before.selection && pane.cursor === before.cursor) return;
    const items = currentItems();
    if (!items) return closeQuickLook();
    const key = JSON.stringify(items);
    if (key === lastSent) return;
    lastSent = key;
    invoke("quicklook_update", items);
  });
  listen<{ delta: number }>("quicklook://navigate", (e) => {
    if (!open) return;
    const s = useApp.getState();
    s.moveCursor(s.activePane, e.payload.delta, false);
  });
  listen("quicklook://closed", () => {
    open = false;
  });
}
