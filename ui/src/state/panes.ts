import { create } from "zustand";
import { api, toFsError, type FileEntry, type FileKind, type FsError } from "../api";

export type PaneId = "left" | "right";
export type SortKey = "name" | "modified" | "kind" | "size";
export type SortDir = "asc" | "desc";

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
  activePane: PaneId;
  showHidden: boolean;
  ready: boolean;
  init: () => Promise<void>;
  patchPane: (id: PaneId, patch: Partial<PaneState>) => void;
  navigate: (id: PaneId, path: string) => Promise<void>;
  reload: (id: PaneId) => Promise<void>;
}

export const useApp = create<AppState>((set, get) => ({
  panes: { left: newPane("/"), right: newPane("/") },
  activePane: "left",
  showHidden: false,
  ready: false,

  init: async () => {
    const home = (await api.startupDir()) ?? (await api.homeDir());
    set({ panes: { left: newPane(home), right: newPane(home) }, ready: true });
    await get().reload("left");
  },

  patchPane: (id, patch) => set((s) => ({ panes: { ...s.panes, [id]: { ...s.panes[id], ...patch } } })),

  navigate: async (id, path) => {
    const pane = get().panes[id];
    if (path === pane.path) return get().reload(id);
    const history = [...pane.history.slice(0, pane.historyIndex + 1), path];
    get().patchPane(id, {
      path,
      history,
      historyIndex: history.length - 1,
      selection: new Set(),
      anchor: null,
      cursor: null,
      entries: [],
    });
    await get().reload(id);
  },

  reload: async (id) => {
    const path = get().panes[id].path;
    get().patchPane(id, { loading: true });
    try {
      const entries = await api.listDir(path, get().showHidden);
      if (get().panes[id].path !== path) return;
      get().patchPane(id, { entries, error: null, loading: false });
    } catch (e) {
      if (get().panes[id].path !== path) return;
      get().patchPane(id, { entries: [], error: toFsError(e), loading: false });
    }
  },
}));
