import { memo } from "react";
import { iconUrl, type FileEntry } from "../api";
import { formatDate, formatSize, kindLabel } from "../format";

interface Props {
  entry: FileEntry;
  top: number;
  height: number;
  iconSize: number;
  selected: boolean;
  cursor: boolean;
  folderColumn: string | null;
}

export const FileRow = memo(function FileRow({ entry, top, height, iconSize, selected, cursor, folderColumn }: Props) {
  return (
    <div
      className={`row${selected ? " selected" : ""}${cursor ? " cursor" : ""}${entry.isHidden ? " hidden-entry" : ""}`}
      style={{ transform: `translateY(${top}px)`, height }}
      data-path={entry.path}
      data-drop-target={entry.isDir ? entry.path : undefined}
    >
      <div className="cell name">
        <img className="icon" src={iconUrl(entry.path, iconSize)} width={iconSize} height={iconSize} draggable={false} alt="" />
        <span className="label">{entry.name}</span>
      </div>
      {folderColumn != null && <div className="cell folder">{folderColumn}</div>}
      <div className="cell date">{formatDate(entry.modified)}</div>
      <div className="cell kind">{kindLabel(entry)}</div>
      <div className="cell size">{formatSize(entry)}</div>
    </div>
  );
});
