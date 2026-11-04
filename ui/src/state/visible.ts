import type { FileEntry } from "../api";
import { kindLabel } from "../format";
import type { PaneState, SortDir, SortKey } from "./panes";

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });

export function compareEntries(a: FileEntry, b: FileEntry, key: SortKey, dir: SortDir, foldersFirst: boolean): number {
  // Like Finder, folders stay on top only in name order; other orders mix them with files.
  if (foldersFirst && key === "name" && a.isDir !== b.isDir) return a.isDir ? -1 : 1;
  let c = 0;
  switch (key) {
    case "name":
      c = collator.compare(a.name, b.name);
      break;
    case "modified":
    case "created": {
      const x = a[key];
      const y = b[key];
      if (x == null || y == null) {
        if (x != null) return -1;
        if (y != null) return 1;
      } else c = x - y;
      break;
    }
    case "kind":
      c = collator.compare(kindLabel(a), kindLabel(b)) || collator.compare(a.ext ?? "", b.ext ?? "");
      break;
    case "size":
      c = (a.size ?? 0) - (b.size ?? 0);
      break;
  }
  if (dir === "desc") c = -c;
  return c || collator.compare(a.name, b.name);
}

const cache = new WeakMap<FileEntry[], { sig: string; kinds: unknown; expanded: unknown; children: unknown; result: FileEntry[] }>();

/** (searchResults ?? entries) -> kindFilter -> nameFilter (when not recursive) -> sort, with expanded folders' children under them. */
export function visibleEntries(p: PaneState): FileEntry[] {
  const source = p.searchResults ?? p.entries;
  const sig = `${p.sortKey}|${p.sortDir}|${p.foldersFirst}|${p.recursive}|${p.nameFilter}`;
  const hit = cache.get(source);
  if (hit && hit.sig === sig && hit.kinds === p.kindFilter && hit.expanded === p.expanded && hit.children === p.children) return hit.result;
  const q = p.nameFilter.trim().toLowerCase();
  const level = (list: FileEntry[]) => {
    if (p.kindFilter.size > 0) list = list.filter((e) => p.kindFilter.has(e.kind === "package" ? "other" : e.kind));
    if (q && !p.recursive) list = list.filter((e) => e.name.toLowerCase().includes(q));
    return [...list].sort((a, b) => compareEntries(a, b, p.sortKey, p.sortDir, p.foldersFirst));
  };
  const result: FileEntry[] = [];
  const walk = (list: FileEntry[]) => {
    for (const e of level(list)) {
      result.push(e);
      const kids = p.expanded.has(e.path) ? p.children[e.path] : undefined;
      if (kids) walk(kids);
    }
  };
  if (p.searchResults) result.push(...level(source));
  else walk(source);
  cache.set(source, { sig, kinds: p.kindFilter, expanded: p.expanded, children: p.children, result });
  return result;
}

export function depthOf(root: string, path: string): number {
  const prefix = root === "/" ? "/" : root + "/";
  if (!path.startsWith(prefix)) return 0;
  return path.slice(prefix.length).split("/").length - 1;
}
