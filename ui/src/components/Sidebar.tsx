import { iconUrl } from "../api";
import { showFavoriteMenu } from "../contextmenu";
import { beginDrag, favoriteSource } from "../dnd";
import { baseName } from "../format";
import { useState } from "react";
import { trashPath, useApp } from "../state/panes";

export function Sidebar() {
  const favorites = useApp((s) => s.favorites);
  const missing = useApp((s) => s.missingFavorites);
  const volumes = useApp((s) => s.volumes);
  const current = useApp((s) => s.panes[s.activePane].path);
  const width = useApp((s) => s.sidebarWidth);
  const trash = trashPath();
  const go = (path: string) => {
    const s = useApp.getState();
    if (s.missingFavorites.has(path)) showFavoriteMenu(path);
    else s.navigate(s.activePane, path);
  };
  return (
    <aside className="sidebar" style={{ width }}>
      <div className="sidebar-top" data-tauri-drag-region />
      <div className="sidebar-scroll">
        <Place path="/Applications" label="Applications" current={current} />
        <div className="sidebar-section">Favorites</div>
        {favorites.map((path) => (
          <div
            key={path}
            className={`sidebar-item${missing.has(path) ? " missing" : ""}${current === path ? " current" : ""}`}
            title={missing.has(path) ? `${path} (not found)` : path}
            data-favorite={path}
            data-drop-target={missing.has(path) ? undefined : path}
            onPointerDown={(e) => beginDrag(e, favoriteSource(path))}
            onClick={() => go(path)}
            onContextMenu={(e) => {
              e.preventDefault();
              showFavoriteMenu(path);
            }}
          >
            <img className="icon" src={iconUrl(path, 16)} width={16} height={16} draggable={false} alt="" />
            <span className="label">{baseName(path)}</span>
          </div>
        ))}
        <div className="sidebar-add" data-sidebar-add="true">
          Drop a folder here to add it
        </div>
        <div className="sidebar-section">Locations</div>
        {volumes.map((v) => (
          <div
            key={v.path}
            className={`sidebar-item${current === v.path ? " current" : ""}`}
            title={v.path}
            data-drop-target={v.path}
            onClick={() => useApp.getState().navigate(useApp.getState().activePane, v.path)}
          >
            <img className="icon" src={iconUrl(v.path, 16)} width={16} height={16} draggable={false} alt="" />
            <span className="label">{v.name}</span>
          </div>
        ))}
        <div className="sidebar-gap" />
        <Place path={trash} label="Trash" current={current} />
      </div>
    </aside>
  );
}

function Place({ path, label, current }: { path: string; label: string; current: string }) {
  return (
    <div
      className={`sidebar-item${current === path ? " current" : ""}`}
      title={path}
      data-drop-target={path}
      onClick={() => useApp.getState().navigate(useApp.getState().activePane, path)}
    >
      <img className="icon" src={iconUrl(path, 16)} width={16} height={16} draggable={false} alt="" />
      <span className="label">{label}</span>
    </div>
  );
}

export function SidebarResizer() {
  const [dragging, setDragging] = useState(false);
  return (
    <div
      className={`sidebar-resizer${dragging ? " dragging" : ""}`}
      onPointerDown={(e) => {
        e.preventDefault();
        e.currentTarget.setPointerCapture(e.pointerId);
        setDragging(true);
      }}
      onPointerMove={(e) => {
        if (!dragging) return;
        const left = e.currentTarget.previousElementSibling!.getBoundingClientRect().left;
        useApp.getState().setSidebarWidth(e.clientX - left);
      }}
      onPointerUp={() => setDragging(false)}
      onPointerCancel={() => setDragging(false)}
    />
  );
}
