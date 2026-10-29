import { useEffect } from "react";
import { startSettingsPersistence, useApp } from "./state/panes";
import { Pane } from "./components/Pane";
import { Sidebar } from "./components/Sidebar";
import { GoToFolder } from "./components/GoToFolder";
import { handleKeyDown } from "./keyboard";
import { listen } from "@tauri-apps/api/event";
import type { DirChanged, SearchBatch } from "./api";

export function App() {
  const ready = useApp((s) => s.ready);
  const goToOpen = useApp((s) => s.goToOpen);
  useEffect(() => {
    useApp.getState().init().then(startSettingsPersistence);
    window.addEventListener("keydown", handleKeyDown);
    const onFocus = () => {
      const s = useApp.getState();
      if (s.ready) s.refreshFavorites();
    };
    window.addEventListener("focus", onFocus);
    const unlisten = [
      listen<SearchBatch>("search://results", (e) => useApp.getState().onSearchBatch(e.payload)),
      listen<DirChanged>("dir://changed", (e) => useApp.getState().onDirChanged(e.payload.path)),
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
    </div>
  );
}
