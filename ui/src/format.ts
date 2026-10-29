import type { FileEntry, FileKind } from "./api";

const KIND_NAMES: Record<FileKind, string> = {
  folder: "Folder",
  package: "Package",
  image: "Image",
  document: "Document",
  video: "Movie",
  audio: "Audio",
  archive: "Archive",
  code: "Source",
  other: "File",
};

export function kindLabel(e: FileEntry): string {
  if (e.kind === "folder") return "Folder";
  if (e.kind === "package") return e.ext === "app" ? "Application" : `${(e.ext ?? "").toUpperCase()} Package`;
  if (!e.ext) return e.kind === "other" ? "Document" : KIND_NAMES[e.kind];
  return `${e.ext.toUpperCase()} ${KIND_NAMES[e.kind]}`;
}

const dateFmt = new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" });

export function formatDate(ms: number | null): string {
  return ms == null ? "--" : dateFmt.format(new Date(ms));
}

export function formatSize(e: FileEntry): string {
  if (e.size == null) return "--";
  const n = e.size;
  if (n < 1000) return `${n} bytes`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = n / 1000;
  let i = 0;
  while (v >= 1000 && i < units.length - 1) {
    v /= 1000;
    i++;
  }
  return `${v < 10 ? v.toFixed(1) : Math.round(v)} ${units[i]}`;
}

export function baseName(path: string): string {
  if (path === "/") return "Macintosh HD";
  return path.split("/").filter(Boolean).pop() ?? path;
}

export function parentOf(path: string): string | null {
  if (path === "/") return null;
  const idx = path.replace(/\/+$/, "").lastIndexOf("/");
  return idx <= 0 ? "/" : path.slice(0, idx);
}

export function joinPath(dir: string, name: string): string {
  return dir === "/" ? `/${name}` : `${dir}/${name}`;
}
