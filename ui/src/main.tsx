import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type E = { path: string; name: string; is_dir: boolean };
const log = (msg: string) => invoke("debug_log", { msg });

function SpikePane({ root, id, sel, setSel }: { root: string; id: string; sel: string | null; setSel: (p: string) => void }) {
  const [entries, setEntries] = useState<E[]>([]);
  useEffect(() => { invoke<E[]>("spike_list", { path: root }).then(setEntries); }, [root]);
  return (
    <div data-drop-target={root} data-pane={id} style={{ flex: 1, border: "1px solid #888", margin: 4, padding: 4, minHeight: 300 }}>
      <b>{id}: {root}</b>
      {entries.map((e) => (
        <div key={e.path} data-row={e.path} data-drop-target={e.is_dir ? e.path : undefined}
          onPointerDown={(ev) => startDrag(ev, e.path, setSel)}
          style={{ padding: 4, background: sel === e.path ? "#3b82f6" : undefined, color: sel === e.path ? "white" : undefined }}>
          {e.is_dir ? "📁" : "📄"} {e.name}
        </div>
      ))}
    </div>
  );
}

function startDrag(ev: React.PointerEvent, path: string, setSel: (p: string) => void) {
  setSel(path);
  const sx = ev.clientX, sy = ev.clientY;
  let ghost: HTMLDivElement | null = null;
  let over: HTMLElement | null = null;
  const move = (m: PointerEvent) => {
    if (!ghost && Math.hypot(m.clientX - sx, m.clientY - sy) > 4) {
      ghost = document.createElement("div");
      ghost.textContent = path.split("/").pop()!;
      Object.assign(ghost.style, { position: "fixed", pointerEvents: "none", background: "#fffa", padding: "2px 6px", border: "1px solid #333" });
      document.body.appendChild(ghost);
    }
    if (!ghost) return;
    ghost.style.left = m.clientX + 8 + "px"; ghost.style.top = m.clientY + 8 + "px";
    ghost.textContent = (m.altKey ? "+ " : "") + path.split("/").pop();
    const t = (document.elementFromPoint(m.clientX, m.clientY) as HTMLElement | null)?.closest("[data-drop-target]") as HTMLElement | null;
    if (over !== t) { if (over) over.style.outline = ""; over = t; if (over) over.style.outline = "2px solid red"; }
  };
  const up = (u: PointerEvent) => {
    window.removeEventListener("pointermove", move); window.removeEventListener("pointerup", up);
    if (over) over.style.outline = "";
    if (ghost) { ghost.remove(); log(`DROP src=${path} target=${over?.dataset.dropTarget ?? "none"} alt=${u.altKey}`); }
  };
  window.addEventListener("pointermove", move); window.addEventListener("pointerup", up);
}

function App() {
  const [sel, setSel] = useState<string | null>(null);
  const [files, setFiles] = useState<string[]>([]);
  const [qlOpen, setQlOpen] = useState(false);
  useEffect(() => { invoke<E[]>("spike_list", { path: "/tmp/rfinder-spike/left" }).then((es) => setFiles(es.filter((e) => !e.is_dir).map((e) => e.path))); }, []);
  useEffect(() => {
    const a = listen<{ delta: number }>("quicklook://navigate", (e) => {
      log(`QL navigate delta=${e.payload.delta}`);
      setSel((cur) => {
        const i = Math.max(0, Math.min(files.length - 1, files.indexOf(cur ?? "") + e.payload.delta));
        invoke("quicklook_update", { paths: [files[i]], index: 0 });
        return files[i];
      });
    });
    const b = listen("quicklook://closed", () => { log("QL closed"); setQlOpen(false); });
    const c = listen<{ paths: string[] }>("tauri://drag-drop", (e) => log(`OS DROP ${JSON.stringify(e.payload)}`));
    return () => { a.then((f) => f()); b.then((f) => f()); c.then((f) => f()); };
  }, [files]);
  useEffect(() => {
    const k = (e: KeyboardEvent) => {
      if (e.key === " " && sel) {
        e.preventDefault();
        if (qlOpen) { invoke("quicklook_close"); setQlOpen(false); }
        else { log(`QL show ${sel}`); invoke("quicklook_show", { paths: [sel], index: 0 }); setQlOpen(true); }
      }
    };
    window.addEventListener("keydown", k);
    return () => window.removeEventListener("keydown", k);
  }, [sel, qlOpen]);
  return (
    <div style={{ display: "flex", fontFamily: "system-ui", userSelect: "none" }}>
      <SpikePane root="/tmp/rfinder-spike/left" id="L" sel={sel} setSel={setSel} />
      <SpikePane root="/tmp/rfinder-spike/right" id="R" sel={sel} setSel={setSel} />
    </div>
  );
}

createRoot(document.getElementById("root")!).render(<App />);
