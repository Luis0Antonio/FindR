import { useEffect } from "react";
import { useApp } from "./state/panes";
import { Pane } from "./components/Pane";

export function App() {
  const ready = useApp((s) => s.ready);
  useEffect(() => {
    useApp.getState().init();
  }, []);
  if (!ready) return null;
  return (
    <div className="app">
      <main className="panes">
        <Pane id="left" />
      </main>
    </div>
  );
}
