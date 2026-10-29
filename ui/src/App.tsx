import { useEffect } from "react";
import { startSettingsPersistence, useApp } from "./state/panes";
import { Pane } from "./components/Pane";
import { Sidebar } from "./components/Sidebar";
import { GoToFolder } from "./components/GoToFolder";
import { ConflictDialog } from "./components/ConflictDialog";
import { handleKeyDown } from "./keyboard";
import { startQuickLookSync } from "./quicklook";
import { listen } from "@tauri-apps/api/event";
import type { DirChanged, OpProgress, SearchBatch } from "./api";

export function App() {
  const ready = useApp((s) => s.ready);
  const goToOpen = useApp((s) => s.goToOpen);
  const notice = useApp((s) => s.notice);
  useEffect(() => {
    useApp.getState().init().then(() => {
      startSettingsPersistence();
      startQuickLookSync();
    });
    window.addEventListener("keydown", handleKeyDown);
    const onFocus = () => {
      const s = useApp.getState();
      if (s.ready) s.refreshFavorites();
    };
    window.addEventListener("focus", onFocus);
    const unlisten = [
      listen<SearchBatch>("search://results", (e) => useApp.getState().onSearchBatch(e.payload)),
      listen<DirChanged>("dir://changed", (e) => useApp.getState().onDirChanged(e.payload.path)),
      listen<OpProgress>("op://progress", (e) =>
        useApp.setState({ progress: e.payload.doneBytes >= e.payload.totalBytes ? null : { done: e.payload.doneBytes, total: e.payload.totalBytes } }),
      ),
    ];
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("focus", onFocus);
      unlisten.forEach((u) => u.then((f) => f()));
    };
  }, []);
  if (!ready) return null;
  return (
    <div className="app">
      <Sidebar />
      <main className="panes">
        <Pane id="left" />
      </main>
      {goToOpen && <GoToFolder />}
      <ConflictDialog />
      {notice && (
        <div className="notice" role="alert">
          <span>{notice}</span>
          <button onClick={() => useApp.setState({ notice: null })}>Dismiss</button>
        </div>
      )}
    </div>
  );
}
