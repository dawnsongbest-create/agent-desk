import { useState } from "react";

type CompactTabProps = {
  onDesktop(): void;
  onOpenReader(): void;
};

export function CompactTab({ onDesktop, onOpenReader }: CompactTabProps) {
  const [expanded, setExpanded] = useState(false);

  return (
    <main className="compact-workspace-stage" aria-label="Agent Desk Compact Tab">
      <section className="compact-entry-paper" data-expanded={expanded}>
        <span className="tape compact-entry-tape" aria-hidden="true" />
        <button
          className="compact-corner-control"
          type="button"
          aria-label="缩小到桌面便利贴"
          onClick={onDesktop}
        />
        <button
          className="compact-entry-expand"
          type="button"
          aria-label="展开 Compact Tab"
          aria-expanded={expanded}
          onClick={() => setExpanded(true)}
        >
          <span>AGENT DESK</span>
          <strong>你的阅读工作空间</strong>
          {expanded ? <p>需要时再打开，平时安静留在桌面。</p> : null}
        </button>
        <button className="compact-reader-entry" type="button" onClick={onOpenReader}>
          打开阅读器
          <span aria-hidden="true">→</span>
        </button>
      </section>
    </main>
  );
}
