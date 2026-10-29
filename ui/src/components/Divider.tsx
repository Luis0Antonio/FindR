import { useRef, useState } from "react";
import { useApp } from "../state/panes";

export function Divider() {
  const ref = useRef<HTMLDivElement>(null);
  const [dragging, setDragging] = useState(false);
  return (
    <div
      ref={ref}
      className={`divider${dragging ? " dragging" : ""}`}
      onPointerDown={(e) => {
        e.preventDefault();
        e.currentTarget.setPointerCapture(e.pointerId);
        setDragging(true);
      }}
      onPointerMove={(e) => {
        if (!dragging) return;
        const rect = ref.current!.parentElement!.getBoundingClientRect();
        useApp.getState().setSplitRatio((e.clientX - rect.left) / rect.width);
      }}
      onPointerUp={() => setDragging(false)}
      onPointerCancel={() => setDragging(false)}
    />
  );
}
