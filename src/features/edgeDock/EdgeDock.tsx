import { useEffect, useRef, useState } from "react";
import type { DesktopStickyWindowPort } from "../../application/ports/desktopSticky";
import { tauriDesktopSticky } from "../../infrastructure/tauri/desktopSticky";

type EdgeDockProps = {
  windowPort?: DesktopStickyWindowPort;
};

export function EdgeDock({ windowPort = tauriDesktopSticky }: EdgeDockProps) {
  const [expanded, setExpanded] = useState(false);
  const collapseTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(
    () => () => {
      if (collapseTimer.current) clearTimeout(collapseTimer.current);
    },
    [],
  );

  function expand() {
    if (collapseTimer.current) clearTimeout(collapseTimer.current);
    collapseTimer.current = null;
    setExpanded(true);
    void windowPort.setEdgeDockExpanded(true).catch(() => undefined);
  }

  function scheduleCollapse() {
    if (collapseTimer.current) clearTimeout(collapseTimer.current);
    collapseTimer.current = setTimeout(() => {
      collapseTimer.current = null;
      setExpanded(false);
      void windowPort.setEdgeDockExpanded(false).catch(() => undefined);
    }, 260);
  }

  return (
    <main
      className="edge-dock-stage"
      data-expanded={expanded}
      aria-label="Agent Desk Edge Dock"
      onMouseEnter={expand}
      onMouseLeave={scheduleCollapse}
      onFocus={expand}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget)) scheduleCollapse();
      }}
    >
      <section className="edge-dock-paper">
        <span className="edge-dock-pin" aria-hidden="true">
          ◆
        </span>
        <div className="edge-dock-entry" aria-hidden={!expanded}>
          <span>Agent Desk</span>
          <button
            type="button"
            tabIndex={expanded ? 0 : -1}
            onClick={() => void windowPort.openReader().catch(() => undefined)}
          >
            打开阅读器
          </button>
        </div>
      </section>
    </main>
  );
}
