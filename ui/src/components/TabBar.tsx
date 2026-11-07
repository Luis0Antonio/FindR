import { useEffect, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { api, iconUrl } from "../api";
import { useApp, currentTabs, displayName, type Tab } from "../state/panes";

const paths = (t: Tab) => (t.splitEnabled ? [t.panes.left.path, t.panes.right.path] : [t.panes.left.path]);

const specialIcons = new Map<string, Promise<boolean>>();

function FolderName({ path }: { path: string }) {
  const [special, setSpecial] = useState(false);
  useEffect(() => {
    let live = true;
    setSpecial(false);
    if (!specialIcons.has(path)) specialIcons.set(path, api.hasSpecialIcon(path).catch(() => false));
    specialIcons.get(path)!.then((yes) => live && setSpecial(yes));
    return () => {
      live = false;
    };
  }, [path]);
  return (
    <span className="tab-folder">
      {special && <img className="icon" src={iconUrl(path, 16)} width={16} height={16} draggable={false} alt="" />}
      <span className="tab-title">{displayName(path)}</span>
    </span>
  );
}

export function TabBar() {
  const tabs = useApp(useShallow((s) => currentTabs(s).map((t) => paths(t).join("\n"))));
  const activeTab = useApp((s) => s.activeTab);
  const s = useApp.getState();
  return (
    <div className="tabbar" data-tauri-drag-region>
      {tabs.map((tip, i) => (
        <div
          key={i}
          className={`tab${i === activeTab ? " active" : ""}`}
          title={tip}
          onPointerDown={(e) => e.button === 0 && s.selectTab(i)}
          onAuxClick={(e) => e.button === 1 && s.closeTab(i)}
        >
          <button
            className="tab-close"
            title="Close Tab"
            onPointerDown={(e) => e.stopPropagation()}
            onClick={() => s.closeTab(i)}
          >
            <svg width="8" height="8" viewBox="0 0 8 8" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round">
              <path d="M1 1l6 6M7 1 1 7" />
            </svg>
          </button>
          {tip.split("\n").map((path, j) => (
            <span key={j} className="tab-part">
              {j > 0 && <span className="tab-sep">|</span>}
              <FolderName path={path} />
            </span>
          ))}
        </div>
      ))}
      <button className="tab-new" title="New Tab (⌘T)" onClick={() => s.newTab()}>
        <svg width="10" height="10" viewBox="0 0 10 10" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round">
          <path d="M5 1v8M1 5h8" />
        </svg>
      </button>
    </div>
  );
}
