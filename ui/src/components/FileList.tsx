import { useEffect, useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { FileEntry } from "../api";
import { useApp, type PaneId, type PaneState, type SortKey } from "../state/panes";
import { FileRow } from "./FileRow";
import { showBackgroundMenu, showRowMenu } from "../contextmenu";
import { beginDrag, itemSource } from "../dnd";
import { depthOf } from "../state/visible";

export function rowHeight(itemSize: number) {
  return Math.max(22, itemSize + 6);
}

export const COLUMN_INFO: Record<SortKey, { label: string; cls: string; width: string }> = {
  name: { label: "Name", cls: "name", width: "minmax(120px, 1fr)" },
  modified: { label: "Date Modified", cls: "date", width: "minmax(0, 170px)" },
  created: { label: "Date Created", cls: "date", width: "minmax(0, 170px)" },
  kind: { label: "Kind", cls: "kind", width: "minmax(0, 130px)" },
  size: { label: "Size", cls: "size", width: "minmax(0, 90px)" },
};

function relativeFolder(root: string, path: string): string {
  const dir = path.slice(0, path.lastIndexOf("/")) || "/";
  if (dir === root) return ".";
  const prefix = root === "/" ? "/" : root + "/";
  return dir.startsWith(prefix) ? dir.slice(prefix.length) : dir;
}

export function FileList({ id, pane, entries }: { id: PaneId; pane: PaneState; entries: FileEntry[] }) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const height = rowHeight(pane.itemSize);
  const showFolder = pane.searchResults != null;
  const columns = useApp((s) => s.columns);
  const cutPaths = useApp((s) => (s.clipboard?.cut ? s.clipboard.paths : null));
  const grid = [COLUMN_INFO.name.width, ...(showFolder ? ["minmax(60px, 0.7fr)"] : []), ...columns.map((c) => COLUMN_INFO[c].width)].join(" ");
  const virtualizer = useVirtualizer({
    count: entries.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => height,
    overscan: 12,
  });
  useEffect(() => virtualizer.measure(), [height, virtualizer]);

  useEffect(() => {
    if (!pane.cursor) return;
    const idx = entries.findIndex((e) => e.path === pane.cursor);
    if (idx >= 0) virtualizer.scrollToIndex(idx, { align: "auto" });
  }, [pane.cursor, entries, virtualizer]);

  const entryAt = (target: EventTarget) => {
    const row = (target as HTMLElement).closest<HTMLElement>(".rows .row");
    if (!row) return null;
    return entries.find((e) => e.path === row.dataset.path) ?? null;
  };

  const onPointerDown = (ev: React.PointerEvent) => {
    if (ev.button !== 0) return;
    const s = useApp.getState();
    const entry = entryAt(ev.target);
    if (entry && (ev.target as HTMLElement).closest(".disclosure")) {
      s.toggleExpand(id, entry.path, ev.altKey);
      return;
    }
    if (!entry) {
      if (!ev.metaKey && !ev.shiftKey) s.clearSelection(id);
      return;
    }
    if (ev.shiftKey) s.selectRange(id, entry.path);
    else if (ev.metaKey) s.toggleSelect(id, entry.path);
    else if (!pane.selection.has(entry.path)) s.selectOnly(id, entry.path);
    if (pane.renaming !== entry.path) beginDrag(ev, itemSource(id));
  };

  const onClick = (ev: React.MouseEvent) => {
    const entry = entryAt(ev.target);
    if (!entry || (ev.target as HTMLElement).closest(".disclosure") || ev.metaKey || ev.shiftKey || ev.detail > 1) return;
    if (pane.selection.size > 1 && pane.selection.has(entry.path)) useApp.getState().selectOnly(id, entry.path);
  };

  const onDoubleClick = (ev: React.MouseEvent) => {
    const entry = entryAt(ev.target);
    if (entry && pane.renaming !== entry.path && !(ev.target as HTMLElement).closest(".disclosure")) useApp.getState().openEntries(id, [entry]);
  };

  const onContextMenu = (ev: React.MouseEvent) => {
    ev.preventDefault();
    const entry = entryAt(ev.target);
    if (entry) showRowMenu(id, entry);
    else showBackgroundMenu(id);
  };

  const headerCell = (key: SortKey) => (
    <div key={key} className={`cell ${COLUMN_INFO[key].cls}${pane.sortKey === key ? " sorted" : ""}`} onClick={() => useApp.getState().setSort(id, key)}>
      {COLUMN_INFO[key].label}
      {pane.sortKey === key && <span className="sort-arrow">{pane.sortDir === "asc" ? "▲" : "▼"}</span>}
    </div>
  );

  return (
    <div className="file-list" style={{ "--columns": grid } as React.CSSProperties}>
      <div className="header row">
        {headerCell("name")}
        {showFolder && <div className="cell folder">Folder</div>}
        {columns.map(headerCell)}
      </div>
      <div className="scroll" ref={scrollRef} onPointerDown={onPointerDown} onClick={onClick} onDoubleClick={onDoubleClick} onContextMenu={onContextMenu} data-pane-scroll={id} data-drop-target={pane.path}>
        <div className="rows" style={{ height: Math.max(virtualizer.getTotalSize(), 1) }}>
          {virtualizer.getVirtualItems().map((v) => {
            const e = entries[v.index];
            return (
              <FileRow
                key={e.path}
                entry={e}
                top={v.start}
                height={height}
                iconSize={pane.itemSize}
                selected={pane.selection.has(e.path)}
                cursor={pane.cursor === e.path}
                folderColumn={showFolder ? relativeFolder(pane.path, e.path) : null}
                depth={showFolder ? null : depthOf(pane.path, e.path)}
                expanded={pane.expanded.has(e.path)}
                columns={columns}
                paneId={id}
                renaming={pane.renaming === e.path}
                cut={cutPaths?.includes(e.path) ?? false}
              />
            );
          })}
        </div>
        {entries.length === 0 && <EmptyState pane={pane} />}
      </div>
    </div>
  );
}

function EmptyState({ pane }: { pane: PaneState }) {
  const query = pane.nameFilter.trim();
  const searching = pane.recursive && query !== "";
  if (pane.loading || (searching && !pane.searchDone)) return null;
  let text = "This folder is empty";
  if (searching) text = `No results for “${query}” in this folder or its subfolders`;
  else if (query || pane.kindFilter.size > 0) text = "No items match the current filter";
  return <div className="empty-state">{text}</div>;
}
