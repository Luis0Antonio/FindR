import { Menu, type MenuItemOptions, type SubmenuOptions } from "@tauri-apps/api/menu";
import { api, type FileEntry } from "./api";
import { useApp, type PaneId } from "./state/panes";
import { visibleEntries } from "./state/visible";
import { isQuickLookOpen, toggleQuickLook } from "./quicklook";
import { convertEntries, convertTargets } from "./convert";

type Item = { text: string; action: () => void; enabled?: boolean } | { text: string; items: Item[] } | "-";

function toMenuItem(i: Item): MenuItemOptions | SubmenuOptions | { item: "Separator" } {
  if (i === "-") return { item: "Separator" };
  if ("items" in i) return { text: i.text, items: i.items.map(toMenuItem) as SubmenuOptions["items"] };
  return { text: i.text, enabled: i.enabled ?? true, action: i.action };
}

async function popup(items: Item[]) {
  const menu = await Menu.new({ items: items.map(toMenuItem) });
  await menu.popup();
}

export function showRowMenu(id: PaneId, entry: FileEntry) {
  const s = useApp.getState();
  if (!s.panes[id].selection.has(entry.path)) s.selectOnly(id, entry.path);
  const pane = useApp.getState().panes[id];
  const selected = visibleEntries(pane).filter((e) => pane.selection.has(e.path));
  const single = selected.length === 1;
  const targets = convertTargets(selected);
  popup([
    { text: "Open", action: () => s.openEntries(id, selected) },
    { text: "Quick Look", action: () => !isQuickLookOpen() && toggleQuickLook() },
    "-",
    { text: "Rename", enabled: single, action: () => s.startRename(id, entry.path) },
    { text: "Add to Sidebar", enabled: single && entry.isDir, action: () => s.addFavorite(entry.path) },
    { text: "Show in Finder", action: () => api.revealInFinder(selected.map((e) => e.path)) },
    { text: "Move to Trash", action: () => s.trashSelection(id) },
    ...(targets.length
      ? ["-" as const, { text: "Convert to", items: targets.map((t) => ({ text: `.${t}`, action: () => convertEntries(id, selected, t) })) }]
      : []),
    "-",
    { text: "New Folder", action: () => s.createFolder(id) },
  ]);
}

export function showBackgroundMenu(id: PaneId) {
  const s = useApp.getState();
  s.clearSelection(id);
  popup([
    { text: "New Folder", action: () => s.createFolder(id) },
    { text: "Add Current Folder to Sidebar", action: () => s.addFavorite(s.panes[id].path) },
    { text: "Show in Finder", action: () => api.openPath(s.panes[id].path) },
  ]);
}

export function showFavoriteMenu(path: string) {
  const s = useApp.getState();
  popup([
    { text: "Open", enabled: !s.missingFavorites.has(path), action: () => s.navigate(s.activePane, path) },
    { text: "Show in Finder", enabled: !s.missingFavorites.has(path), action: () => api.revealInFinder([path]) },
    "-",
    { text: "Remove from Sidebar", action: () => s.removeFavorite(path) },
  ]);
}
