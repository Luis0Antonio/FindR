import { api, type FileEntry } from "./api";
import { baseName, parentOf } from "./format";
import { useApp, type PaneId } from "./state/panes";
import { visibleEntries } from "./state/visible";
import { transfer } from "./transfer";

const THRESHOLD = 4;
const SPRING_MS = 1000;

type Source = { kind: "items"; paneId: PaneId; entries: FileEntry[] } | { kind: "favorite"; path: string };

type Target =
  | { kind: "folder"; path: string; el: HTMLElement }
  | { kind: "add"; el: HTMLElement }
  | { kind: "reorder"; index: number; el: HTMLElement; after: boolean };

const volumes = new Map<string, number>();

function volumeOf(path: string): number | undefined {
  if (!volumes.has(path)) {
    volumes.set(path, -1);
    api.volumeOf(path).then((v) => volumes.set(path, v), () => volumes.delete(path));
  }
  const v = volumes.get(path);
  return v === -1 ? undefined : v;
}

function invalidReason(entries: FileEntry[], target: string): string | null {
  if (entries.every((e) => parentOf(e.path) === target)) return "already here";
  for (const e of entries) {
    if (e.path === target || target.startsWith(e.path + "/")) return "into itself";
  }
  return null;
}

function ghostLabel(source: Source) {
  if (source.kind === "favorite") return baseName(source.path);
  return source.entries.length === 1 ? source.entries[0].name : `${source.entries.length} items`;
}

export function beginDrag(ev: React.PointerEvent, makeSource: () => Source | null) {
  if (ev.button !== 0) return;
  const startX = ev.clientX;
  const startY = ev.clientY;
  const pointerId = ev.pointerId;
  let source: Source | null = null;
  let ghost: HTMLDivElement | null = null;
  let badge: HTMLSpanElement | null = null;
  let target: Target | null = null;
  let invalid = false;
  let alt = ev.altKey;
  let springTimer: number | undefined;
  let springEl: HTMLElement | null = null;
  let lastX = startX;
  let lastY = startY;

  const clearTarget = () => {
    target?.el.classList.remove("drop-hover", "drop-before", "drop-after");
    document.body.classList.remove("drop-invalid");
    target = null;
  };

  const updateBadge = () => {
    if (!badge || !source) return;
    let text = "";
    if (source.kind === "items" && target?.kind === "folder" && !invalid) {
      if (alt) text = "+ Copy";
      else {
        const src = volumeOf(parentOf(source.entries[0].path) ?? "/");
        const dst = volumeOf(target.path);
        text = src !== undefined && dst !== undefined && src !== dst ? "Move to volume" : "";
      }
    }
    badge.textContent = text;
    badge.style.display = text ? "inline-block" : "none";
  };

  const resolve = (x: number, y: number) => {
    clearTarget();
    invalid = false;
    const hit = document.elementFromPoint(x, y) as HTMLElement | null;
    if (!hit || !source) return;
    if (source.kind === "favorite") {
      const item = hit.closest<HTMLElement>("[data-favorite]");
      if (!item) return;
      const favs = useApp.getState().favorites;
      const rect = item.getBoundingClientRect();
      const after = y > rect.top + rect.height / 2;
      let index = favs.indexOf(item.dataset.favorite!) + (after ? 1 : 0);
      if (favs.indexOf(source.path) < index) index--;
      target = { kind: "reorder", index, el: item, after };
      item.classList.add(after ? "drop-after" : "drop-before");
      return;
    }
    const add = hit.closest<HTMLElement>("[data-sidebar-add]");
    if (add) {
      if (source.entries.some((e) => e.isDir)) {
        target = { kind: "add", el: add };
        add.classList.add("drop-hover");
      }
      return;
    }
    const el = hit.closest<HTMLElement>("[data-drop-target]");
    if (!el) return;
    const path = el.dataset.dropTarget!;
    target = { kind: "folder", path, el };
    invalid = invalidReason(source.entries, path) !== null;
    if (invalid) document.body.classList.add("drop-invalid");
    else el.classList.add("drop-hover");
  };

  const spring = () => {
    window.clearTimeout(springTimer);
    const el = target?.kind === "folder" && !invalid && target.el.classList.contains("row") ? target.el : null;
    springEl = el;
    if (!el) return;
    springTimer = window.setTimeout(() => {
      if (springEl !== el || target?.el !== el) return;
      const paneId = el.closest<HTMLElement>("[data-pane-scroll]")?.dataset.paneScroll as PaneId | undefined;
      if (paneId) {
        clearTarget();
        useApp.getState().navigate(paneId, el.dataset.dropTarget!).then(() => requestAnimationFrame(() => refresh()));
      }
    }, SPRING_MS);
  };

  const refresh = () => {
    const prevEl = target?.el;
    resolve(lastX, lastY);
    if (target?.el !== prevEl) spring();
    updateBadge();
  };

  const start = () => {
    source = makeSource();
    if (!source) return false;
    ghost = document.createElement("div");
    ghost.className = "drag-ghost";
    const label = document.createElement("span");
    label.textContent = ghostLabel(source);
    badge = document.createElement("span");
    badge.className = "drag-badge";
    ghost.append(label, badge);
    document.body.appendChild(ghost);
    document.body.classList.add("dragging");
    if (source.kind === "items") {
      if (source.entries.some((e) => e.isDir)) document.body.classList.add("dragging-folder");
      volumeOf(parentOf(source.entries[0].path) ?? "/");
    }
    try {
      document.body.setPointerCapture(pointerId);
    } catch {
      /* the pointer may already be released */
    }
    return true;
  };

  const onMove = (e: PointerEvent) => {
    lastX = e.clientX;
    lastY = e.clientY;
    alt = e.altKey;
    if (!ghost) {
      if (Math.hypot(e.clientX - startX, e.clientY - startY) <= THRESHOLD) return;
      if (!start()) return cleanup();
    }
    ghost!.style.transform = `translate(${e.clientX + 12}px, ${e.clientY + 10}px)`;
    refresh();
  };

  const onKey = (e: KeyboardEvent) => {
    if (!ghost) return;
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      clearTarget();
      cleanup();
      return;
    }
    if (e.key === "Alt") {
      alt = e.type === "keydown";
      updateBadge();
    }
  };

  const onUp = (e: PointerEvent) => {
    const dragged = !!ghost;
    const drop = target;
    const wasInvalid = invalid;
    const copy = e.altKey || alt;
    const src = source;
    clearTarget();
    cleanup();
    if (!dragged || !src || !drop || wasInvalid) return;
    window.addEventListener("click", swallow, { capture: true, once: true });
    if (src.kind === "favorite" && drop.kind === "reorder") {
      useApp.getState().moveFavorite(src.path, drop.index);
    } else if (src.kind === "items" && drop.kind === "add") {
      for (const e of src.entries) if (e.isDir) useApp.getState().addFavorite(e.path);
    } else if (src.kind === "items" && drop.kind === "folder") {
      transfer(
        src.entries.map((x) => x.path),
        drop.path,
        copy,
      );
    }
  };

  const swallow = (e: Event) => {
    e.stopPropagation();
    e.preventDefault();
  };

  const cleanup = () => {
    window.clearTimeout(springTimer);
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onUp);
    window.removeEventListener("keydown", onKey, true);
    window.removeEventListener("keyup", onKey, true);
    ghost?.remove();
    ghost = null;
    document.body.classList.remove("dragging", "dragging-folder", "drop-invalid");
  };

  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onUp);
  window.addEventListener("keydown", onKey, true);
  window.addEventListener("keyup", onKey, true);
}

export function itemSource(paneId: PaneId): () => Source | null {
  return () => {
    const pane = useApp.getState().panes[paneId];
    if (pane.renaming) return null;
    const entries = visibleEntries(pane).filter((e) => pane.selection.has(e.path));
    return entries.length ? { kind: "items", paneId, entries } : null;
  };
}

export function favoriteSource(path: string): () => Source {
  return () => ({ kind: "favorite", path });
}
