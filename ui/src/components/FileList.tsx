import { useEffect, useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { FileEntry } from "../api";
import type { PaneState } from "../state/panes";
import { FileRow } from "./FileRow";

export function rowHeight(itemSize: number) {
  return Math.max(22, itemSize + 6);
}

export function FileList({ pane, entries }: { pane: PaneState; entries: FileEntry[] }) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const height = rowHeight(pane.itemSize);
  const virtualizer = useVirtualizer({
    count: entries.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => height,
    overscan: 12,
  });
  useEffect(() => virtualizer.measure(), [height, virtualizer]);

  return (
    <div className="file-list">
      <div className="header row">
        <div className="cell name">Name</div>
        <div className="cell date">Date Modified</div>
        <div className="cell kind">Kind</div>
        <div className="cell size">Size</div>
      </div>
      <div className="scroll" ref={scrollRef}>
        <div className="rows" style={{ height: virtualizer.getTotalSize() }}>
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
              />
            );
          })}
        </div>
      </div>
    </div>
  );
}
