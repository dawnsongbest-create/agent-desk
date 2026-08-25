import { listen } from "@tauri-apps/api/event";
import { useEffect, useMemo, useRef, useState, type PointerEvent } from "react";
import type { DesktopStickyWindowPort } from "../../application/ports/desktopSticky";
import type { Preferences } from "../../domain/preferences";
import { defaultPreferences } from "../../domain/preferences";
import { recordExcerpt } from "../../domain/sticky";
import type { StickyCardsPort } from "../../application/ports/sticky";
import { tauriDesktopSticky } from "../../infrastructure/tauri/desktopSticky";
import { tauriPreferences } from "../../infrastructure/tauri/preferences";
import { tauriStickyCards } from "../../infrastructure/tauri/sticky";
import { applyTheme } from "../../styles/theme";
import { useStickyCards } from "../sticky/useStickyCards";
import "../../App.css";

type DesktopStickyProps = {
  stickyPort?: StickyCardsPort;
  windowPort?: DesktopStickyWindowPort;
};

export function DesktopSticky({
  stickyPort = tauriStickyCards,
  windowPort = tauriDesktopSticky,
}: DesktopStickyProps) {
  const sticky = useStickyCards(stickyPort);
  const retrySticky = sticky.retry;
  const [preferences, setPreferences] = useState<Preferences>(defaultPreferences);
  const clickTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const gesture = useRef<{ x: number; y: number; dragging: boolean } | null>(null);
  const suppressNextClick = useRef(false);

  useEffect(() => {
    let cancelled = false;
    let unlistenPreferences: () => void = () => undefined;
    let unlistenRefresh: () => void = () => undefined;

    void tauriPreferences.load().then((stored) => {
      if (cancelled) return;
      setPreferences(stored);
      applyTheme(stored.theme);
    });
    void listen<Preferences>("agent-desk://preferences-changed", ({ payload }) => {
      setPreferences(payload);
      applyTheme(payload.theme);
    }).then((dispose) => {
      if (cancelled) dispose();
      else unlistenPreferences = dispose;
    });
    void listen("agent-desk://desktop-sticky-refresh", () => {
      void retrySticky();
    }).then((dispose) => {
      if (cancelled) dispose();
      else unlistenRefresh = dispose;
    });

    return () => {
      cancelled = true;
      unlistenPreferences();
      unlistenRefresh();
      if (clickTimer.current) clearTimeout(clickTimer.current);
    };
  }, [retrySticky]);

  const presentation = useMemo(() => {
    const openTasks = sticky.cards.filter((card) => card.kind === "task" && !card.completed);
    const records = sticky.cards.filter((card) => card.kind === "note");
    switch (preferences.desktopStickyKind) {
      case "record":
        return {
          label: "RECORD",
          count: records.length ? "✎" : "0",
          detail: records.length
            ? recordExcerpt(records[records.length - 1].text, 18)
            : "点击打开记录",
          accessible: records.length
            ? `记录：${recordExcerpt(records[records.length - 1].text, 40)}`
            : "暂无记录",
        };
      case "quote":
        return {
          label: "QUOTE",
          count: "“",
          detail: recordExcerpt(sticky.profile.quoteText || "留一句愿意常常看到的话。", 18),
          accessible: `便签一句：${sticky.profile.quoteText || "暂无内容"}`,
        };
      default:
        return {
          label: "TODAY",
          count: String(openTasks.length),
          detail: openTasks[0]?.text ?? "今天没有未完成待办",
          accessible: `${openTasks.length} 件未完成待办${openTasks[0] ? `：${openTasks[0].text}` : ""}`,
        };
    }
  }, [preferences.desktopStickyKind, sticky.cards, sticky.profile.quoteText]);

  function cancelClick() {
    if (!clickTimer.current) return;
    clearTimeout(clickTimer.current);
    clickTimer.current = null;
  }

  function open(edit = false) {
    cancelClick();
    void windowPort.open(edit ? preferences.desktopStickyKind : undefined);
  }

  function pointerDown(event: PointerEvent<HTMLElement>) {
    if (event.button !== 0) return;
    gesture.current = { x: event.clientX, y: event.clientY, dragging: false };
  }

  function pointerMove(event: PointerEvent<HTMLElement>) {
    const current = gesture.current;
    if (!current || current.dragging) return;
    if (Math.hypot(event.clientX - current.x, event.clientY - current.y) < 4) return;
    current.dragging = true;
    cancelClick();
    void windowPort.startDragging();
  }

  return (
    <main className="desktop-sticky-stage">
      <section
        className="desktop-sticky-paper mini-tab"
        data-kind={preferences.desktopStickyKind}
        role="button"
        tabIndex={0}
        aria-label={`桌面便利贴，${presentation.accessible}`}
        onPointerDown={pointerDown}
        onPointerMove={pointerMove}
        onPointerUp={() => {
          if (gesture.current?.dragging) {
            suppressNextClick.current = true;
            cancelClick();
          }
          gesture.current = null;
        }}
        onClick={() => {
          if (suppressNextClick.current) {
            suppressNextClick.current = false;
            return;
          }
          cancelClick();
          clickTimer.current = setTimeout(() => open(false), 240);
        }}
        onDoubleClick={() => open(true)}
        onContextMenu={(event) => {
          event.preventDefault();
          cancelClick();
          void windowPort.showContextMenu();
        }}
        onKeyDown={(event) => {
          if (event.key === "Enter" || event.key === " ") open(false);
        }}
      >
        <span className="mini-fold" aria-hidden="true" />
        <span className="mini-label">{presentation.label}</span>
        <strong>{presentation.count}</strong>
        <span className="desktop-sticky-detail">{presentation.detail}</span>
      </section>
    </main>
  );
}
