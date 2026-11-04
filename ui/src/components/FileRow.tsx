import { memo } from "react";
import { iconUrl, type Column, type FileEntry } from "../api";
import { formatDate, formatSize, kindLabel } from "../format";
import type { PaneId } from "../state/panes";
import { RenameEditor } from "./RenameEditor";

interface Props {
  entry: FileEntry;
  top: number;
  height: number;
  iconSize: number;
  selected: boolean;
  cursor: boolean;
  folderColumn: string | null;
  depth: number | null;
  expanded: boolean;
  columns: Column[];
  paneId: PaneId;
  renaming: boolean;
  cut: boolean;
}

export const FileRow = memo(function FileRow({ entry, top, height, iconSize, selected, cursor, folderColumn, depth, expanded, columns, paneId, renaming, cut }: Props) {
  return (
    <div
      className={`row${selected ? " selected" : ""}${cursor ? " cursor" : ""}${entry.isHidden ? " hidden-entry" : ""}${cut ? " cut" : ""}`}
      style={{ transform: `translateY(${top}px)`, height }}
      data-path={entry.path}
      data-drop-target={entry.isDir ? entry.path : undefined}
    >
      <div className="cell name" style={depth != null ? { paddingLeft: depth * 16 } : undefined}>
        {depth != null && <span className={`disclosure${entry.isDir ? "" : " none"}${expanded ? " open" : ""}`} />}
        <img className="icon" src={iconUrl(entry.path, iconSize, entry.modified)} width={iconSize} height={iconSize} draggable={false} alt="" />
        {renaming ? <RenameEditor id={paneId} entry={entry} /> : <span className="label">{entry.name}</span>}
      </div>
      {folderColumn != null && <div className="cell folder">{folderColumn}</div>}
      {columns.map((c) => {
        if (c === "kind") return <div key={c} className="cell kind">{kindLabel(entry)}</div>;
        if (c === "size") return <div key={c} className="cell size">{formatSize(entry)}</div>;
        return <div key={c} className="cell date">{formatDate(entry[c])}</div>;
      })}
    </div>
  );
});
