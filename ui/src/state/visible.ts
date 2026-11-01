import type { FileEntry } from "../api";
import { kindLabel } from "../format";
import type { PaneState, SortDir, SortKey } from "./panes";

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });

export function compareEntries(a: FileEntry, b: FileEntry, key: SortKey, dir: SortDir, foldersFirst: boolean): number {
  if (foldersFirst && a.isDir !== b.isDir) return a.isDir ? -1 : 1;
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

const cache = new WeakMap<FileEntry[], { sig: string; kinds: unknown; result: FileEntry[] }>();

/** (searchResults ?? entries) -> kindFilter -> nameFilter (when not recursive) -> sort. */
export function visibleEntries(p: PaneState): FileEntry[] {
  const source = p.searchResults ?? p.entries;
  const sig = `${p.sortKey}|${p.sortDir}|${p.foldersFirst}|${p.recursive}|${p.nameFilter}`;
  const hit = cache.get(source);
  if (hit && hit.sig === sig && hit.kinds === p.kindFilter) return hit.result;
  let list = source;
  if (p.kindFilter.size > 0) list = list.filter((e) => p.kindFilter.has(e.kind === "package" ? "other" : e.kind));
  const q = p.nameFilter.trim().toLowerCase();
  if (q && !p.recursive) list = list.filter((e) => e.name.toLowerCase().includes(q));
  const result = [...list].sort((a, b) => compareEntries(a, b, p.sortKey, p.sortDir, p.foldersFirst));
  cache.set(source, { sig, kinds: p.kindFilter, result });
  return result;
}
