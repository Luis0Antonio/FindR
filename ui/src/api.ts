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

export const api = {
  listDir: (path: string, showHidden: boolean) => invoke<FileEntry[]>("list_dir", { path, showHidden }),
  openPath: (path: string) => invoke<void>("open_path", { path }),
  homeDir: () => invoke<string>("home_dir"),
  startupDir: () => invoke<string | null>("startup_dir"),
  openPrivacySettings: () => invoke<void>("open_privacy_settings"),
  log: (msg: string) => invoke<void>("debug_log", { msg }),
};

export function iconUrl(path: string, size: number): string {
  return `icon://localhost/${size}/${encodeURIComponent(path)}`;
}
