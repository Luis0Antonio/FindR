import { useEffect, useRef, useState } from "react";
import type { FileEntry } from "../api";
import { useApp, type PaneId } from "../state/panes";

export function RenameEditor({ id, entry }: { id: PaneId; entry: FileEntry }) {
  const ref = useRef<HTMLInputElement>(null);
  const [error, setError] = useState<string | null>(null);
  const busy = useRef(false);

  useEffect(() => {
    const input = ref.current;
    if (!input) return;
    input.focus();
    const dot = entry.name.lastIndexOf(".");
    const keepExt = entry.kind !== "folder" && entry.kind !== "package" && dot > 0;
    input.setSelectionRange(0, keepExt ? dot : entry.name.length);
  }, [entry]);

  const commit = async () => {
    if (busy.current || !ref.current) return;
    busy.current = true;
    const err = await useApp.getState().commitRename(id, ref.current.value);
    busy.current = false;
    if (err) {
      setError(err.message);
      ref.current?.focus();
    }
  };

  return (
    <span className="rename-wrap">
      <input
        ref={ref}
        className={`rename-input${error ? " invalid" : ""}`}
        defaultValue={entry.name}
        spellCheck={false}
        autoCorrect="off"
        autoCapitalize="off"
        onPointerDown={(e) => e.stopPropagation()}
        onDoubleClick={(e) => e.stopPropagation()}
        onChange={() => setError(null)}
        onKeyDown={(e) => {
          e.stopPropagation();
          if (e.key === "Enter") {
            e.preventDefault();
            commit();
          } else if (e.key === "Escape") {
            e.preventDefault();
            busy.current = true;
            useApp.getState().cancelRename(id);
          }
        }}
        onBlur={() => {
          if (!error) commit();
        }}
      />
      {error && <span className="rename-error">{error}</span>}
    </span>
  );
}
