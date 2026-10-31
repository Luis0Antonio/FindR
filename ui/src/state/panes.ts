import { create } from "zustand";
import { api, toFsError, type ConflictPolicy, type FileEntry, type FileKind, type FsError, type SearchBatch, type Settings, type Volume } from "../api";
import { parentOf } from "../format";
import { visibleEntries } from "./visible";

export type PaneId = "left" | "right";
export type SortKey = "name" | "modified" | "kind" | "size";
export type SortDir = "asc" | "desc";

export const MIN_ITEM = 16;
export const MAX_ITEM = 64;

export interface PaneState {
  path: string;
  history: string[];
  historyIndex: number;
  entries: FileEntry[];
  sortKey: SortKey;
  sortDir: SortDir;
  foldersFirst: boolean;
  nameFilter: string;
  kindFilter: Set<FileKind>;
  recursive: boolean;
  searchResults: FileEntry[] | null;
  searchId: string | null;
  searchDone: boolean;
  searchTruncated: boolean;
  selection: Set<string>;
  anchor: string | null;
  cursor: string | null;
  renaming: string | null;
  itemSize: number;
  error: FsError | null;
  loading: boolean;
}

export function newPane(path: string): PaneState {
  return {
    path,
    history: [path],
    historyIndex: 0,
    entries: [],
    sortKey: "name",
    sortDir: "asc",
    foldersFirst: true,
    nameFilter: "",
    kindFilter: new Set(),
    recursive: false,
    searchResults: null,
    searchId: null,
    searchDone: true,
    searchTruncated: false,
    selection: new Set(),
    anchor: null,
    cursor: null,
    renaming: null,
    itemSize: 20,
    error: null,
    loading: false,
  };
}

interface AppState {
  panes: Record<PaneId, PaneState>;
  splitEnabled: boolean;
  splitRatio: number;
  activePane: PaneId;
  showHidden: boolean;
  favorites: string[];
  home: string;
  ready: boolean;
  goToOpen: boolean;
  volumes: Volume[];
  missingFavorites: Set<string>;
  conflictPrompt: ConflictPrompt | null;
  notice: string | null;
  activity: string | null;
  progress: { done: number; total: number } | null;

  init: () => Promise<void>;
  patchPane: (id: PaneId, patch: Partial<PaneState>) => void;
  setActive: (id: PaneId) => void;
  toggleSplit: () => void;
  openRightPane: () => void;
  closePane: (id: PaneId) => void;
  setSplitRatio: (ratio: number) => void;
  navigate: (id: PaneId, path: string, opts?: { select?: string; history?: "push" | "keep" }) => Promise<void>;
  reload: (id: PaneId, opts?: { fromWatcher?: boolean }) => Promise<void>;
  back: (id: PaneId) => void;
  forward: (id: PaneId) => void;
  up: (id: PaneId) => void;
  openEntries: (id: PaneId, entries: FileEntry[]) => void;
  openSelection: (id: PaneId) => void;
  setSort: (id: PaneId, key: SortKey) => void;
  setItemSize: (id: PaneId, size: number) => void;
  selectOnly: (id: PaneId, path: string) => void;
  toggleSelect: (id: PaneId, path: string) => void;
  selectRange: (id: PaneId, path: string) => void;
  selectAll: (id: PaneId) => void;
  clearSelection: (id: PaneId) => void;
  moveCursor: (id: PaneId, delta: number, extend: boolean) => void;
  setGoToOpen: (open: boolean) => void;
  setNameFilter: (id: PaneId, text: string) => void;
  setRecursive: (id: PaneId, on: boolean) => void;
  toggleKind: (id: PaneId, kind: FileKind) => void;
  clearKinds: (id: PaneId) => void;
  toggleHidden: () => void;
  runSearch: (id: PaneId) => Promise<void>;
  stopSearch: (id: PaneId) => void;
  onSearchBatch: (batch: SearchBatch) => void;
  onDirChanged: (path: string) => void;
  startRename: (id: PaneId, path?: string) => void;
  cancelRename: (id: PaneId) => void;
  commitRename: (id: PaneId, newName: string) => Promise<FsError | null>;
  createFolder: (id: PaneId) => Promise<void>;
  trashSelection: (id: PaneId) => Promise<void>;
  addFavorite: (path: string) => void;
  removeFavorite: (path: string) => void;
  moveFavorite: (path: string, toIndex: number) => void;
  refreshFavorites: () => Promise<void>;
  loadVolumes: () => Promise<void>;
}

export interface ConflictPrompt {
  name: string;
  remaining: number;
  resolve: (choice: { policy: ConflictPolicy; all: boolean }) => void;
}

const refreshAfterRename: Partial<Record<PaneId, boolean>> = {};

const searchTimers: Partial<Record<PaneId, number>> = {};
const SEARCH_DEBOUNCE_MS = 200;
let searchCounter = 0;

export const useApp = create<AppState>((set, get) => ({
  panes: { left: newPane("/"), right: newPane("/") },
  splitEnabled: false,
  splitRatio: 0.5,
  activePane: "left",
  showHidden: false,
  favorites: [],
  home: "/",
  ready: false,
  goToOpen: false,
  volumes: [],
  missingFavorites: new Set(),
  conflictPrompt: null,
  notice: null,
  activity: null,
  progress: null,

  init: async () => {
    const [home, startup, saved] = await Promise.all([api.homeDir(), api.startupDir(), api.loadSettings().catch(() => null)]);
    const pane = (id: PaneId): PaneState => {
      const ps = saved?.panes[id];
      const p = newPane(ps?.path ?? home);
      if (ps) Object.assign(p, { sortKey: ps.sortKey, sortDir: ps.sortDir, foldersFirst: ps.foldersFirst, itemSize: ps.itemSize });
      return p;
    };
    const panes = { left: pane("left"), right: pane("right") };
    if (startup) panes.left = { ...panes.left, path: startup, history: [startup] };
    set({
      home,
      panes,
      splitEnabled: saved?.splitEnabled ?? false,
      splitRatio: saved?.splitRatio ?? 0.5,
      showHidden: saved?.showHidden ?? false,
      favorites: saved?.favorites ?? [home, `${home}/Desktop`, `${home}/Documents`, `${home}/Downloads`],
      ready: true,
    });
    await Promise.all([get().reload("left"), get().splitEnabled ? get().reload("right") : null]);
    get().refreshFavorites();
    get().loadVolumes();
    api.watchDir("volumes", "/Volumes");
  },

  patchPane: (id, patch) => set((s) => ({ panes: { ...s.panes, [id]: { ...s.panes[id], ...patch } } })),

  setActive: (id) => {
    if (get().activePane !== id) set({ activePane: id });
  },

  toggleSplit: () => {
    const s = get();
    if (s.splitEnabled) {
      s.stopSearch("right");
      api.unwatchDir("right");
      set({ splitEnabled: false, activePane: "left" });
      return;
    }
    const { path } = s.panes.left;
    const right = s.panes.right;
    set({
      splitEnabled: true,
      panes: {
        ...s.panes,
        right: { ...newPane(path), sortKey: right.sortKey, sortDir: right.sortDir, foldersFirst: right.foldersFirst, itemSize: right.itemSize },
      },
    });
    get().reload("right");
  },

  openRightPane: () => {
    if (!get().splitEnabled) get().toggleSplit();
    set({ activePane: "right" });
  },

  closePane: (id) => {
    const s = get();
    if (!s.splitEnabled) return;
    if (id === "right") return s.toggleSplit();
    s.stopSearch("left");
    s.stopSearch("right");
    api.unwatchDir("right");
    set({ splitEnabled: false, activePane: "left", panes: { ...s.panes, left: get().panes.right } });
    get().reload("left");
    get().runSearch("left");
  },

  setSplitRatio: (ratio) => set({ splitRatio: Math.min(0.8, Math.max(0.2, ratio)) }),

  navigate: async (id, path, opts) => {
    const pane = get().panes[id];
    const select = opts?.select ?? null;
    if (path !== pane.path) {
      let { history, historyIndex } = pane;
      if (opts?.history !== "keep") {
        history = [...history.slice(0, historyIndex + 1), path];
        historyIndex = history.length - 1;
      }
      get().patchPane(id, {
        path,
        history,
        historyIndex,
        entries: [],
        nameFilter: "",
        renaming: null,
      });
      get().stopSearch(id);
    }
    get().patchPane(id, { selection: new Set(select ? [select] : []), anchor: select, cursor: select });
    await get().reload(id);
  },

  reload: async (id, opts) => {
    const path = get().panes[id].path;
    get().patchPane(id, { loading: true });
    api.watchDir(id, path);
    try {
      const entries = await api.listDir(path, get().showHidden);
      const pane = get().panes[id];
      if (pane.path !== path) return;
      const present = new Set(entries.map((e) => e.path));
      const selection = new Set([...pane.selection].filter((p) => present.has(p)));
      get().patchPane(id, {
        entries,
        error: null,
        loading: false,
        selection,
        cursor: pane.cursor && present.has(pane.cursor) ? pane.cursor : null,
        anchor: pane.anchor && present.has(pane.anchor) ? pane.anchor : null,
      });
    } catch (e) {
      if (get().panes[id].path !== path) return;
      const error = toFsError(e);
      if (error.code === "not_found" && opts?.fromWatcher) {
        for (let dir = parentOf(path); dir; dir = parentOf(dir)) {
          const ok = await api.listDir(dir, false).then(() => true, () => false);
          if (ok) return get().navigate(id, dir, { history: "keep" });
        }
      }
      get().patchPane(id, { entries: [], error, loading: false });
    }
  },

  back: (id) => {
    const p = get().panes[id];
    if (p.historyIndex <= 0) return;
    const from = p.path;
    get().patchPane(id, { historyIndex: p.historyIndex - 1 });
    const target = p.history[p.historyIndex - 1];
    get().navigate(id, target, { history: "keep", select: parentOf(from) === target ? from : undefined });
  },

  forward: (id) => {
    const p = get().panes[id];
    if (p.historyIndex >= p.history.length - 1) return;
    get().patchPane(id, { historyIndex: p.historyIndex + 1 });
    get().navigate(id, p.history[p.historyIndex + 1], { history: "keep" });
  },

  up: (id) => {
    const p = get().panes[id];
    const parent = parentOf(p.path);
    if (parent) get().navigate(id, parent, { select: p.path });
  },

  openEntries: (id, entries) => {
    if (entries.length === 1 && entries[0].isDir) {
      get().navigate(id, entries[0].path);
      return;
    }
    for (const e of entries) if (!e.isDir) api.openPath(e.path);
  },

  openSelection: (id) => {
    const p = get().panes[id];
    get().openEntries(id, visibleEntries(p).filter((e) => p.selection.has(e.path)));
  },

  setSort: (id, key) => {
    const p = get().panes[id];
    get().patchPane(id, p.sortKey === key ? { sortDir: p.sortDir === "asc" ? "desc" : "asc" } : { sortKey: key, sortDir: "asc" });
  },

  setItemSize: (id, size) => get().patchPane(id, { itemSize: Math.max(MIN_ITEM, Math.min(MAX_ITEM, Math.round(size))) }),

  selectOnly: (id, path) => get().patchPane(id, { selection: new Set([path]), anchor: path, cursor: path }),

  toggleSelect: (id, path) => {
    const sel = new Set(get().panes[id].selection);
    if (sel.has(path)) sel.delete(path);
    else sel.add(path);
    get().patchPane(id, { selection: sel, anchor: path, cursor: path });
  },

  selectRange: (id, path) => {
    const p = get().panes[id];
    const list = visibleEntries(p);
    const anchor = p.anchor ?? path;
    const a = list.findIndex((e) => e.path === anchor);
    const b = list.findIndex((e) => e.path === path);
    if (a < 0 || b < 0) return get().selectOnly(id, path);
    const [lo, hi] = a < b ? [a, b] : [b, a];
    get().patchPane(id, { selection: new Set(list.slice(lo, hi + 1).map((e) => e.path)), cursor: path, anchor });
  },

  selectAll: (id) => {
    const list = visibleEntries(get().panes[id]);
    get().patchPane(id, { selection: new Set(list.map((e) => e.path)) });
  },

  clearSelection: (id) => get().patchPane(id, { selection: new Set(), anchor: null, cursor: null }),

  moveCursor: (id, delta, extend) => {
    const p = get().panes[id];
    const list = visibleEntries(p);
    if (list.length === 0) return;
    const cur = p.cursor ? list.findIndex((e) => e.path === p.cursor) : -1;
    const next = cur < 0 ? (delta > 0 ? 0 : list.length - 1) : Math.max(0, Math.min(list.length - 1, cur + delta));
    const path = list[next].path;
    if (extend) get().selectRange(id, path);
    else get().selectOnly(id, path);
  },

  setGoToOpen: (open) => set({ goToOpen: open }),

  setNameFilter: (id, text) => {
    get().patchPane(id, { nameFilter: text });
    if (get().panes[id].recursive) {
      window.clearTimeout(searchTimers[id]);
      searchTimers[id] = window.setTimeout(() => get().runSearch(id), SEARCH_DEBOUNCE_MS);
    }
  },

  setRecursive: (id, on) => {
    get().patchPane(id, { recursive: on });
    if (on) get().runSearch(id);
    else get().stopSearch(id);
  },

  toggleKind: (id, kind) => {
    const kinds = new Set(get().panes[id].kindFilter);
    if (kinds.has(kind)) kinds.delete(kind);
    else kinds.add(kind);
    get().patchPane(id, { kindFilter: kinds });
    if (get().panes[id].recursive) get().runSearch(id);
  },

  clearKinds: (id) => {
    get().patchPane(id, { kindFilter: new Set() });
    if (get().panes[id].recursive) get().runSearch(id);
  },

  toggleHidden: () => {
    set({ showHidden: !get().showHidden });
    for (const id of ["left", "right"] as PaneId[]) {
      if (id === "right" && !get().splitEnabled) continue;
      get().reload(id);
      if (get().panes[id].recursive) get().runSearch(id);
    }
  },

  runSearch: async (id) => {
    get().stopSearch(id);
    const p = get().panes[id];
    const query = p.nameFilter.trim();
    if (!p.recursive || !query) {
      get().patchPane(id, { searchResults: null, searchId: null, searchDone: true, searchTruncated: false });
      return;
    }
    const searchId = `${id}-${++searchCounter}`;
    get().patchPane(id, { searchResults: [], searchId, searchDone: false, searchTruncated: false });
    await api.startSearch(searchId, p.path, query, [...p.kindFilter], get().showHidden);
  },

  stopSearch: (id) => {
    window.clearTimeout(searchTimers[id]);
    const p = get().panes[id];
    if (p.searchId) api.cancelSearch(p.searchId);
    get().patchPane(id, { searchId: null, searchResults: null, searchDone: true, searchTruncated: false });
  },

  onSearchBatch: (batch) => {
    for (const id of ["left", "right"] as PaneId[]) {
      const p = get().panes[id];
      if (p.searchId !== batch.searchId) continue;
      get().patchPane(id, {
        searchResults: batch.entries.length ? [...(p.searchResults ?? []), ...batch.entries] : p.searchResults,
        searchDone: batch.done,
        searchTruncated: batch.truncated ?? p.searchTruncated,
      });
    }
  },

  onDirChanged: (path) => {
    if (path === "/Volumes") {
      get().loadVolumes();
      return;
    }
    for (const id of ["left", "right"] as PaneId[]) {
      if (id === "right" && !get().splitEnabled) continue;
      if (get().panes[id].path !== path) continue;
      if (get().panes[id].renaming) refreshAfterRename[id] = true;
      else get().reload(id, { fromWatcher: true });
    }
  },

  startRename: (id, path) => {
    const p = get().panes[id];
    const target = path ?? (p.selection.size === 1 ? [...p.selection][0] : null);
    if (!target) return;
    get().patchPane(id, { renaming: target, selection: new Set([target]), cursor: target, anchor: target });
  },

  cancelRename: (id) => {
    get().patchPane(id, { renaming: null });
    if (refreshAfterRename[id]) {
      refreshAfterRename[id] = false;
      get().reload(id);
    }
  },

  commitRename: async (id, newName) => {
    const p = get().panes[id];
    const path = p.renaming;
    if (!path) return null;
    const old = (p.searchResults ?? p.entries).find((e) => e.path === path);
    if (!old || newName === old.name) {
      get().cancelRename(id);
      return null;
    }
    try {
      const updated = await api.rename(path, newName);
      const swap = (list: FileEntry[]) => list.map((e) => (e.path === path ? updated : e));
      const now = get().panes[id];
      get().patchPane(id, {
        entries: swap(now.entries),
        searchResults: now.searchResults ? swap(now.searchResults) : null,
        selection: new Set([updated.path]),
        cursor: updated.path,
        anchor: updated.path,
      });
      if (get().favorites.includes(path)) set({ favorites: get().favorites.map((f) => (f === path ? updated.path : f)) });
      get().cancelRename(id);
      return null;
    } catch (e) {
      return toFsError(e);
    }
  },

  createFolder: async (id) => {
    const p = get().panes[id];
    try {
      const entry = await api.createFolder(p.path);
      const now = get().panes[id];
      get().patchPane(id, {
        entries: [...now.entries.filter((e) => e.path !== entry.path), entry],
        searchResults: now.searchResults ? [...now.searchResults, entry] : null,
        selection: new Set([entry.path]),
        cursor: entry.path,
        anchor: entry.path,
        renaming: entry.path,
      });
    } catch (e) {
      get().patchPane(id, { error: toFsError(e) });
    }
  },

  trashSelection: async (id) => {
    const pane = get().panes[id];
    const visible = visibleEntries(pane);
    const doomed = visible.filter((e) => pane.selection.has(e.path)).map((e) => e.path);
    if (!doomed.length) return;
    const lastIndex = visible.findIndex((e) => e.path === doomed[doomed.length - 1]);
    const next = visible.slice(lastIndex + 1).find((e) => !pane.selection.has(e.path)) ?? [...visible.slice(0, lastIndex)].reverse().find((e) => !pane.selection.has(e.path));
    try {
      await api.moveToTrash(doomed);
    } catch (e) {
      set({ notice: toFsError(e).message });
    }
    await get().reload(id);
    const other: PaneId = id === "left" ? "right" : "left";
    if (get().splitEnabled && get().panes[other].path === pane.path) get().reload(other);
    const stillThere = next && visibleEntries(get().panes[id]).some((e) => e.path === next.path);
    if (stillThere) get().selectOnly(id, next.path);
    else get().clearSelection(id);
  },

  addFavorite: (path) => {
    if (get().favorites.includes(path)) return;
    set({ favorites: [...get().favorites, path] });
    get().refreshFavorites();
  },

  removeFavorite: (path) => set({ favorites: get().favorites.filter((f) => f !== path) }),

  moveFavorite: (path, toIndex) => {
    const list = get().favorites.filter((f) => f !== path);
    list.splice(Math.max(0, Math.min(list.length, toIndex)), 0, path);
    set({ favorites: list });
  },

  refreshFavorites: async () => {
    const favs = get().favorites;
    const exist = await api.pathsExist(favs);
    set({ missingFavorites: new Set(favs.filter((_, i) => !exist[i])) });
  },

  loadVolumes: async () => set({ volumes: await api.listVolumes() }),
}));

export function activePaneState() {
  const s = useApp.getState();
  return { id: s.activePane, pane: s.panes[s.activePane] };
}

let saveTimer: number | undefined;
let lastSaved = "";

export function startSettingsPersistence() {
  useApp.subscribe((s) => {
    if (!s.ready) return;
    const pane = (p: PaneState) => ({
      path: p.path,
      sortKey: p.sortKey,
      sortDir: p.sortDir,
      foldersFirst: p.foldersFirst,
      itemSize: p.itemSize,
    });
    const settings: Settings = {
      favorites: s.favorites,
      splitEnabled: s.splitEnabled,
      splitRatio: s.splitRatio,
      showHidden: s.showHidden,
      panes: { left: pane(s.panes.left), right: pane(s.panes.right) },
    };
    const text = JSON.stringify(settings);
    if (text === lastSaved) return;
    window.clearTimeout(saveTimer);
    saveTimer = window.setTimeout(() => {
      lastSaved = text;
      api.saveSettings(settings);
    }, 300);
  });
}
