import { useEffect } from "react";
import { startSettingsPersistence, useApp } from "./state/panes";
import { Pane } from "./components/Pane";
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
    const unlisten = [
      listen<SearchBatch>("search://results", (e) => useApp.getState().onSearchBatch(e.payload)),
      listen<DirChanged>("dir://changed", (e) => useApp.getState().onDirChanged(e.payload.path)),
    ];
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      unlisten.forEach((u) => u.then((f) => f()));
    };
  }, []);
  if (!ready) return null;
  return (
    <div className="app">
      <main className="panes">
        <Pane id="left" />
      </main>
      {goToOpen && <GoToFolder />}
    </div>
  );
}
