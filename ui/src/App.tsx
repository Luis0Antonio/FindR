import { useEffect } from "react";
import { startSettingsPersistence, useApp } from "./state/panes";
import { Pane } from "./components/Pane";
import { GoToFolder } from "./components/GoToFolder";
import { handleKeyDown } from "./keyboard";

export function App() {
  const ready = useApp((s) => s.ready);
  const goToOpen = useApp((s) => s.goToOpen);
  useEffect(() => {
    useApp.getState().init().then(startSettingsPersistence);
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
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
