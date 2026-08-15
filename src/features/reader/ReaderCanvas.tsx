import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties } from "react";
import ReactMarkdown, { type Components } from "react-markdown";
import type {
  ReaderFontSize,
  ReaderLineSpacing,
  ReaderSkin,
  WindowPreset,
} from "../../domain/preferences";
import type { ReaderDocument } from "../../domain/reader";
import type { ReaderLoadState } from "./useReaderDocument";

type ReaderCanvasProps = {
  skin: ReaderSkin;
  fontSize: ReaderFontSize;
  lineSpacing: ReaderLineSpacing;
  windowPreset: WindowPreset;
  document: ReaderDocument | null;
  state: ReaderLoadState;
  contentVisible: boolean;
  onRetry(): void;
  onContentVisibilityChange(visible: boolean): void;
  onCopy(text: string): Promise<void>;
  onCaptureSelection(documentId: string, text: string): Promise<boolean>;
};

type SelectionStatus = "ready" | "copying" | "copied" | "saving" | "saved" | "error";

type SelectionPopover = {
  text: string;
  anchor: DOMRect;
  left: number;
  top: number;
  status: SelectionStatus;
};

const markdownComponents: Components = {
  a: ({ children, href }) => (
    <a href={href} target="_blank" rel="noreferrer">
      {children}
    </a>
  ),
};

const documentTypeLabels: Record<ReaderDocument["documentType"], string> = {
  article: "文章",
  brief: "简报",
  reading: "阅读",
  report: "报告",
};

function placePopover(anchor: DOMRect, width = 188, height = 38) {
  const edge = 8;
  const left = Math.min(
    Math.max(edge, anchor.left + anchor.width / 2 - width / 2),
    Math.max(edge, window.innerWidth - width - edge),
  );
  const above = anchor.top - height - edge;
  const top =
    above >= edge ? above : Math.min(anchor.bottom + edge, window.innerHeight - height - edge);
  return { left, top: Math.max(edge, top) };
}

export function ReaderCanvas({
  skin,
  fontSize,
  lineSpacing,
  windowPreset,
  document,
  state,
  contentVisible,
  onRetry,
  onContentVisibilityChange,
  onCopy,
  onCaptureSelection,
}: ReaderCanvasProps) {
  const contentRef = useRef<HTMLElement>(null);
  const scrollViewportRef = useRef<HTMLDivElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const closeTimerRef = useRef<number | null>(null);
  const hiddenScrollTopRef = useRef(0);
  const restoreScrollRef = useRef(false);
  const [popover, setPopover] = useState<SelectionPopover | null>(null);

  function closePopover() {
    if (closeTimerRef.current !== null) window.clearTimeout(closeTimerRef.current);
    closeTimerRef.current = null;
    setPopover(null);
  }

  function detectSelection() {
    const content = contentRef.current;
    const selection = window.getSelection();
    if (
      !content ||
      !selection ||
      selection.rangeCount === 0 ||
      selection.isCollapsed ||
      !selection.anchorNode ||
      !selection.focusNode ||
      !content.contains(selection.anchorNode) ||
      !content.contains(selection.focusNode)
    ) {
      closePopover();
      return;
    }
    const text = selection.toString();
    if (!text.trim()) {
      closePopover();
      return;
    }
    if (closeTimerRef.current !== null) window.clearTimeout(closeTimerRef.current);
    closeTimerRef.current = null;
    const range = selection.getRangeAt(0);
    const anchor = range.getBoundingClientRect();
    const position = placePopover(anchor);
    setPopover({ text, anchor, ...position, status: "ready" });
  }

  useEffect(() => {
    function selectionChanged() {
      const selection = window.getSelection();
      if (!selection || selection.isCollapsed || !selection.toString().trim()) closePopover();
    }
    function resized() {
      closePopover();
    }
    globalThis.document.addEventListener("selectionchange", selectionChanged);
    window.addEventListener("resize", resized);
    return () => {
      globalThis.document.removeEventListener("selectionchange", selectionChanged);
      window.removeEventListener("resize", resized);
      if (closeTimerRef.current !== null) window.clearTimeout(closeTimerRef.current);
    };
  }, [document?.id]);

  useEffect(() => {
    if (contentVisible) return;
    if (closeTimerRef.current !== null) window.clearTimeout(closeTimerRef.current);
    closeTimerRef.current = null;
    setPopover(null);
    window.getSelection()?.removeAllRanges();
  }, [contentVisible]);

  useLayoutEffect(() => {
    if (!contentVisible || !restoreScrollRef.current) return;
    restoreScrollRef.current = false;
    const frame = window.requestAnimationFrame(() => {
      if (scrollViewportRef.current) {
        scrollViewportRef.current.scrollTop = hiddenScrollTopRef.current;
      }
    });
    return () => window.cancelAnimationFrame(frame);
  }, [contentVisible]);

  useLayoutEffect(() => {
    if (!popover || !popoverRef.current) return;
    const bounds = popoverRef.current.getBoundingClientRect();
    const next = placePopover(popover.anchor, bounds.width, bounds.height);
    if (next.left !== popover.left || next.top !== popover.top) {
      setPopover((current) => (current ? { ...current, ...next } : current));
    }
  }, [popover]);

  async function copySelection() {
    if (!popover) return;
    const text = popover.text;
    setPopover((current) => (current ? { ...current, status: "copying" } : current));
    try {
      await onCopy(text);
      setPopover((current) => (current ? { ...current, status: "copied" } : current));
      closeTimerRef.current = window.setTimeout(closePopover, 760);
    } catch {
      setPopover((current) => (current ? { ...current, status: "error" } : current));
    }
  }

  async function saveSelection() {
    if (!popover || !document) return;
    const text = popover.text;
    setPopover((current) => (current ? { ...current, status: "saving" } : current));
    const saved = await onCaptureSelection(document.id, text);
    if (!saved) {
      setPopover((current) => (current ? { ...current, status: "error" } : current));
      return;
    }
    setPopover((current) => (current ? { ...current, status: "saved" } : current));
    closeTimerRef.current = window.setTimeout(closePopover, 920);
  }

  function toggleContentVisibility() {
    if (contentVisible) {
      hiddenScrollTopRef.current = scrollViewportRef.current?.scrollTop ?? 0;
      closePopover();
      window.getSelection()?.removeAllRanges();
      onContentVisibilityChange(false);
      return;
    }
    restoreScrollRef.current = true;
    onContentVisibilityChange(true);
  }

  const popoverStyle: CSSProperties | undefined = popover
    ? { left: popover.left, top: popover.top }
    : undefined;

  return (
    <section
      className="reader-canvas"
      data-reader-layer="1"
      data-reader-skin={skin}
      data-font-size={fontSize}
      data-line-spacing={lineSpacing}
      data-window-preset={windowPreset}
      data-current-document-id={document?.id ?? ""}
      data-content-visible={contentVisible}
      aria-label="Reader Canvas"
    >
      <div className="reader-safe-shelf" data-testid="reader-safe-shelf">
        <button
          className="reader-visibility-control"
          type="button"
          aria-pressed={!contentVisible}
          onClick={toggleContentVisibility}
        >
          {contentVisible ? "隐藏正文" : "显示正文"}
        </button>
        <span className="reader-mini-shelf-slot" data-testid="reader-mini-shelf-slot" aria-hidden />
      </div>
      <div
        ref={scrollViewportRef}
        className="reader-scroll-viewport"
        data-content-visible={contentVisible}
        role="region"
        aria-label="Reader scroll viewport"
        tabIndex={contentVisible ? 0 : -1}
        onScroll={closePopover}
      >
        {contentVisible && state === "error" ? (
          <div className="reader-load-state" role="alert">
            <p>无法读取本地阅读文档。</p>
            <button type="button" onClick={onRetry}>
              重试
            </button>
          </div>
        ) : null}
        {contentVisible && document ? (
          <article
            ref={contentRef}
            className="reader-content"
            aria-label={document.title}
            onMouseUp={detectSelection}
            onKeyUp={detectSelection}
          >
            <header className="reader-document-header">
              <p className="reader-context">
                {documentTypeLabels[document.documentType]} ·{" "}
                {document.sourceLabel ?? document.sourceType}
              </p>
              <h1>{document.title}</h1>
              {document.subtitle ? <p className="reader-subtitle">{document.subtitle}</p> : null}
            </header>
            <div className="reader-article">
              <ReactMarkdown skipHtml components={markdownComponents}>
                {document.contentMarkdown}
              </ReactMarkdown>
            </div>
            <p className="reader-end-mark" aria-hidden="true">
              ✦
            </p>
          </article>
        ) : contentVisible && state === "loading" ? (
          <div className="reader-load-state" aria-label="正在读取文档">
            正在展开纸张…
          </div>
        ) : null}
      </div>
      {contentVisible && popover ? (
        <div
          ref={popoverRef}
          className="reader-selection-popover"
          data-status={popover.status}
          style={popoverStyle}
          role="toolbar"
          aria-label="所选文字操作"
          onPointerDown={(event) => event.preventDefault()}
        >
          {popover.status === "copied" ? <span role="status">已复制</span> : null}
          {popover.status === "saved" ? <span role="status">✓ 已保存到记录</span> : null}
          {popover.status !== "copied" && popover.status !== "saved" ? (
            <>
              <button
                type="button"
                disabled={popover.status === "copying" || popover.status === "saving"}
                onClick={() => void copySelection()}
              >
                {popover.status === "copying" ? "复制中" : "复制"}
              </button>
              <button
                type="button"
                disabled={popover.status === "copying" || popover.status === "saving"}
                onClick={() => void saveSelection()}
              >
                {popover.status === "saving" ? "保存中" : "保存到记录"}
              </button>
              {popover.status === "error" ? <span role="status">操作失败，请重试</span> : null}
            </>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}
