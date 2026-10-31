import { invoke } from "@tauri-apps/api/core";

export type FileKind = "folder" | "package" | "image" | "document" | "video" | "audio" | "archive" | "code" | "other";

export interface FileEntry {
  path: string;
  name: string;
  ext: string | null;
  kind: FileKind;
  isDir: boolean;
  isHidden: boolean;
  isSymlink: boolean;
  size: number | null;
  modified: number | null;
  created: number | null;
}

export interface FsError {
  code: string;
  message: string;
}

export function toFsError(e: unknown): FsError {
  if (e && typeof e === "object" && "code" in e) return e as FsError;
  return { code: "io", message: String(e) };
}

export interface PaneSettings {
  path: string;
  sortKey: "name" | "modified" | "kind" | "size";
  sortDir: "asc" | "desc";
  foldersFirst: boolean;
  itemSize: number;
}

export interface Settings {
  favorites: string[];
  splitEnabled: boolean;
  splitRatio: number;
  showHidden: boolean;
  panes: Record<"left" | "right", PaneSettings>;
}

export const api = {
  listDir: (path: string, showHidden: boolean) => invoke<FileEntry[]>("list_dir", { path, showHidden }),
  openPath: (path: string) => invoke<void>("open_path", { path }),
  homeDir: () => invoke<string>("home_dir"),
  startupDir: () => invoke<string | null>("startup_dir"),
  openPrivacySettings: () => invoke<void>("open_privacy_settings"),
  loadSettings: () => invoke<Settings | null>("load_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  startSearch: (searchId: string, root: string, query: string, kinds: FileKind[], showHidden: boolean) =>
    invoke<string>("start_search", { searchId, root, query, kinds, showHidden }),
  cancelSearch: (searchId: string) => invoke<void>("cancel_search", { searchId }),
  watchDir: (paneId: string, path: string) => invoke<void>("watch_dir", { paneId, path }),
  unwatchDir: (paneId: string) => invoke<void>("unwatch_dir", { paneId }),
  createFolder: (parent: string) => invoke<FileEntry>("create_folder", { parent }),
  rename: (path: string, newName: string) => invoke<FileEntry>("rename", { path, newName }),
  listVolumes: () => invoke<Volume[]>("list_volumes"),
  pathsExist: (paths: string[]) => invoke<boolean[]>("paths_exist", { paths }),
  moveItems: (sources: string[], targetDir: string, copy: boolean, policy: ConflictPolicy | null, opId: string) =>
    invoke<OpResult>("move_items", { sources, targetDir, copy, policy, opId }),
  volumeOf: (path: string) => invoke<number>("volume_of", { path }),
  convertFile: (path: string, target: string) => invoke<string>("convert_file", { path, target }),
  dragOut: (paths: string[]) => invoke<void>("drag_out", { paths }),
  revealInFinder: (paths: string[]) => invoke<void>("reveal_in_finder", { paths }),
};

export function iconUrl(path: string, size: number, modified?: number | null): string {
  const url = `icon://localhost/${size}/${encodeURIComponent(path)}`;
  return modified == null ? url : `${url}?m=${modified}`;
}

export interface SearchBatch {
  searchId: string;
  entries: FileEntry[];
  done: boolean;
  truncated?: boolean;
}

export interface DirChanged {
  paneId: string;
  path: string;
}

export interface Volume {
  name: string;
  path: string;
}

export type ConflictPolicy = "replace" | "keepBoth" | "skip";

export interface OpResult {
  done: string[];
  conflicts: { source: string; target: string }[];
  errors: { path: string; message: string }[];
}

export interface OpProgress {
  opId: string;
  doneBytes: number;
  totalBytes: number;
}
