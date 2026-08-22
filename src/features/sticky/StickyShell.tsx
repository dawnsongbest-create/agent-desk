import {
  closestCenter,
  DndContext,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type PointerEvent,
  type ReactNode,
} from "react";
import type {
  Preferences,
  ReaderFontSize,
  ReaderLineSpacing,
  StickyMode,
  StickyPosition,
  ThemeMode,
  WindowPreset,
} from "../../domain/preferences";
import type { ReaderDocument } from "../../domain/reader";
import type { InboxDelivery } from "../../domain/delivery";
import type { CreateReadingPlanInput, ReadingPlan, ReadingPlanStatus } from "../../domain/reading";
import type { AgentProposal } from "../../domain/proposal";
import type { AgentBridgeStatus } from "../../domain/agentConnection";
import {
  compactDragFrame,
  readerFontSizes,
  readerLineSpacings,
  settleCompactPosition,
  themeModes,
  windowPresets,
} from "../../domain/preferences";
import {
  formatLocalDueDate,
  parseCapture,
  recordDisplayTitle,
  recordExcerpt,
  reorderCardIdsWithinKind,
  type CreateStickyCardInput,
  type StickyCard,
  type StickyProfile,
} from "../../domain/sticky";
import { ReaderCanvas } from "../reader/ReaderCanvas";
import type { InboxLoadState } from "../inbox/useInbox";
import type { ReaderLoadState } from "../reader/useReaderDocument";
import type { ReadingPlansState } from "../reading/useReadingPlans";
import { playPageTurnSound } from "./pageTurnSound";
import type { StickyLoadState } from "./useStickyCards";
import { AgentBridgeSettings } from "../agentBridge/AgentBridgeSettings";
import type { AgentBridgeLoadState } from "../agentBridge/useAgentBridge";

type SaveState = "loading" | "idle" | "saving" | "saved" | "error";
type StickyFace = "note" | "task";
type ReaderSurfaceMode = "reader" | "inbox" | "plans";

type StickyShellProps = {
  preferences: Preferences;
  readerDocument: ReaderDocument | null;
  readerState: ReaderLoadState;
  inboxItems: InboxDelivery[];
  inboxUnreadCount: number;
  inboxState: InboxLoadState;
  inboxOpeningId: string | null;
  inboxOpenError: string | null;
  readingPlans: ReadingPlan[];
  readingPlansState: ReadingPlansState;
  readingPlansError: string | null;
  readingBusyPlanId: string | null;
  agentProposals: AgentProposal[];
  proposalBusyId: string | null;
  proposalErrorId: string | null;
  agentBridgeStatus: AgentBridgeStatus | null;
  agentBridgeState: AgentBridgeLoadState;
  issuedAgentToken: string | null;
  agentTokenCopied: boolean;
  preferenceSaveState: SaveState;
  cards: StickyCard[];
  profile: StickyProfile;
  stickyState: StickyLoadState;
  error: string | null;
  now?: Date;
  onThemeChange(theme: ThemeMode): void;
  onAlwaysOnTopChange(alwaysOnTop: boolean): void;
  onWindowPresetChange(preset: WindowPreset): void;
  onStickyPositionChange(position: StickyPosition): void;
  onStickyModeChange(mode: StickyMode): void;
  onReaderFontSizeChange(size: ReaderFontSize): void;
  onReaderLineSpacingChange(spacing: ReaderLineSpacing): void;
  onReaderContentVisibilityChange(visible: boolean): void;
  onRetryReader(): void;
  onRetryInbox(): void;
  onOpenDelivery(id: string): Promise<boolean>;
  onCopyReaderSelection(text: string): Promise<void>;
  onCaptureReaderSelection(documentId: string, text: string): Promise<boolean>;
  onCreateReadingSession(documentId: string, text: string): Promise<boolean>;
  onRetryReadingPlans(): void;
  onCreateReadingPlan(input: CreateReadingPlanInput): Promise<boolean>;
  onGenerateReadingDelivery(id: string): Promise<boolean>;
  onSetReadingPlanStatus(id: string, status: ReadingPlanStatus): Promise<void>;
  onAcceptProposal(id: string): Promise<boolean>;
  onRejectProposal(id: string): Promise<boolean>;
  onAgentBridgeEnabledChange(enabled: boolean): Promise<void>;
  onGenerateAgentToken(): Promise<void>;
  onCopyAgentToken(): Promise<void>;
  onRetryAgentBridge(): Promise<void>;
  onCreate(input: CreateStickyCardInput): Promise<boolean>;
  onUpdateText(id: string, text: string): Promise<boolean>;
  onTaskCompleted(id: string, completed: boolean): Promise<void>;
  onTaskDueDate(id: string, dueDate: string | null): Promise<void>;
  onDelete(id: string): Promise<void>;
  onReorder(orderedIds: string[]): Promise<void>;
  onUpdateQuote(text: string): Promise<boolean>;
  onExportRecord(id: string): Promise<boolean>;
  onRetry(): Promise<void>;
  onDismissError(): void;
};

const themeLabels: Record<ThemeMode, string> = {
  system: "跟随系统",
  light: "浅色",
  dark: "深色",
};

const presetLabels: Record<WindowPreset, string> = {
  sticky: "Sticky",
  iphone5: "iPhone 5",
  pocket: "Pocket",
  book: "Book",
  custom: "Custom",
};

const faceLabels: Record<StickyFace, string> = { note: "记录", task: "待办" };
const readerFontSizeLabels: Record<ReaderFontSize, string> = {
  small: "小",
  standard: "标准",
  large: "大",
};
const readerLineSpacingLabels: Record<ReaderLineSpacing, string> = {
  compact: "紧凑",
  standard: "标准",
  relaxed: "宽松",
};
const SNAP_DISTANCE = 28;

function SettingsMenu({
  preferences,
  saveState,
  agentBridgeStatus,
  agentBridgeState,
  issuedAgentToken,
  agentTokenCopied,
  onThemeChange,
  onAlwaysOnTopChange,
  onWindowPresetChange,
  onReaderFontSizeChange,
  onReaderLineSpacingChange,
  onAgentBridgeEnabledChange,
  onGenerateAgentToken,
  onCopyAgentToken,
  onRetryAgentBridge,
}: {
  preferences: Preferences;
  saveState: SaveState;
  agentBridgeStatus: AgentBridgeStatus | null;
  agentBridgeState: AgentBridgeLoadState;
  issuedAgentToken: string | null;
  agentTokenCopied: boolean;
  onThemeChange(theme: ThemeMode): void;
  onAlwaysOnTopChange(alwaysOnTop: boolean): void;
  onWindowPresetChange(preset: WindowPreset): void;
  onReaderFontSizeChange(size: ReaderFontSize): void;
  onReaderLineSpacingChange(spacing: ReaderLineSpacing): void;
  onAgentBridgeEnabledChange(enabled: boolean): void;
  onGenerateAgentToken(): void;
  onCopyAgentToken(): void;
  onRetryAgentBridge(): void;
}) {
  const [open, setOpen] = useState(false);
  const disabled = saveState === "loading" || saveState === "saving";
  return (
    <div className="settings-anchor">
      <button
        className="board-icon-button settings-button"
        type="button"
        aria-label="外观与窗口设置"
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
      >
        <span aria-hidden="true">•••</span>
      </button>
      {open ? (
        <div className="settings-panel" aria-label="外观与窗口设置面板">
          <div className="settings-heading">
            <strong>外观</strong>
            <span>纸张明暗</span>
          </div>
          <div className="theme-switcher" role="group" aria-label="主题">
            {themeModes.map((theme) => (
              <button
                className="theme-button"
                type="button"
                key={theme}
                disabled={disabled}
                aria-pressed={preferences.theme === theme}
                onClick={() => onThemeChange(theme)}
              >
                {themeLabels[theme]}
              </button>
            ))}
          </div>
          <div className="settings-section reader-settings">
            <div className="settings-heading">
              <strong>Reader</strong>
              <span>阅读字号与行距</span>
            </div>
            <div className="reader-setting-row">
              <span>字号</span>
              <div className="reader-option-grid" role="group" aria-label="Reader 字号">
                {readerFontSizes.map((size) => (
                  <button
                    type="button"
                    key={size}
                    disabled={disabled}
                    aria-pressed={preferences.readerFontSize === size}
                    onClick={() => onReaderFontSizeChange(size)}
                  >
                    {readerFontSizeLabels[size]}
                  </button>
                ))}
              </div>
            </div>
            <div className="reader-setting-row">
              <span>行距</span>
              <div className="reader-option-grid" role="group" aria-label="Reader 行距">
                {readerLineSpacings.map((spacing) => (
                  <button
                    type="button"
                    key={spacing}
                    disabled={disabled}
                    aria-pressed={preferences.readerLineSpacing === spacing}
                    onClick={() => onReaderLineSpacingChange(spacing)}
                  >
                    {readerLineSpacingLabels[spacing]}
                  </button>
                ))}
              </div>
            </div>
          </div>
          <div className="settings-section">
            <div className="settings-heading">
              <strong>窗口尺寸</strong>
              <span>仍可自由调整</span>
            </div>
            <div className="preset-grid" role="group" aria-label="窗口尺寸模板">
              {windowPresets
                .filter((preset) => preset !== "custom")
                .map((preset) => (
                  <button
                    type="button"
                    key={preset}
                    disabled={disabled}
                    aria-pressed={preferences.windowPreset === preset}
                    onClick={() => onWindowPresetChange(preset)}
                  >
                    {presetLabels[preset]}
                  </button>
                ))}
            </div>
            {preferences.windowPreset === "custom" ? <small>当前：Custom</small> : null}
          </div>
          <div className="settings-rule" />
          <div className="settings-section">
            <div className="settings-heading">
              <strong>Agent Bridge</strong>
              <span>本机连接</span>
            </div>
            <AgentBridgeSettings
              status={agentBridgeStatus}
              state={agentBridgeState}
              issuedToken={issuedAgentToken}
              copied={agentTokenCopied}
              onEnabledChange={onAgentBridgeEnabledChange}
              onGenerateToken={onGenerateAgentToken}
              onCopyToken={onCopyAgentToken}
              onRetry={onRetryAgentBridge}
            />
          </div>
          <div className="settings-rule" />
          <button
            className="pin-button"
            type="button"
            disabled={disabled}
            aria-pressed={preferences.alwaysOnTop}
            onClick={() => onAlwaysOnTopChange(!preferences.alwaysOnTop)}
          >
            <span>{preferences.alwaysOnTop ? "取消窗口置顶" : "将窗口置顶"}</span>
            <span aria-hidden="true">{preferences.alwaysOnTop ? "✓" : "○"}</span>
          </button>
        </div>
      ) : null}
    </div>
  );
}

function QuoteEditor({
  profile,
  disabled,
  onUpdate,
}: {
  profile: StickyProfile;
  disabled: boolean;
  onUpdate(text: string): Promise<boolean>;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(profile.quoteText);
  useEffect(() => {
    if (!editing) setDraft(profile.quoteText);
  }, [editing, profile.quoteText]);

  async function save() {
    if (await onUpdate(draft)) setEditing(false);
  }

  return (
    <section className="quote-editor" aria-label="便签一句">
      <div>
        <span>便签一句</span>
        <small>Compact Sticky 会一直显示</small>
      </div>
      {editing ? (
        <div className="quote-composer">
          <textarea
            autoFocus
            value={draft}
            rows={3}
            maxLength={10000}
            aria-label="编辑便签一句"
            placeholder="留一句愿意常常看到的话…"
            onChange={(event) => setDraft(event.target.value)}
          />
          <div>
            <button type="button" onClick={() => setEditing(false)}>
              取消
            </button>
            <button type="button" disabled={disabled} onClick={() => void save()}>
              保存
            </button>
          </div>
        </div>
      ) : (
        <button
          className="quote-value"
          type="button"
          disabled={disabled}
          onClick={() => setEditing(true)}
        >
          {profile.quoteText || "写下你的便签一句"}
        </button>
      )}
    </section>
  );
}

function TaskCapture({
  disabled,
  onCreate,
}: {
  disabled: boolean;
  onCreate(input: CreateStickyCardInput): Promise<boolean>;
}) {
  const [open, setOpen] = useState(false);
  const [text, setText] = useState("");
  const [dueDate, setDueDate] = useState("");
  async function submit() {
    const input = parseCapture(text, "task", dueDate || null);
    if (input && !disabled && (await onCreate(input))) {
      setText("");
      setDueDate("");
      setOpen(false);
    }
  }
  return open ? (
    <div id="task-capture" className="face-capture-composer" data-kind="task">
      <textarea
        autoFocus
        value={text}
        rows={2}
        maxLength={4000}
        placeholder="要完成什么？"
        aria-label="新待办"
        onChange={(event) => setText(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Escape") setOpen(false);
          if (event.key === "Enter" && !event.shiftKey) {
            event.preventDefault();
            void submit();
          }
        }}
      />
      <div className="capture-actions">
        <label className="capture-due">
          <span>日期</span>
          <input type="date" value={dueDate} onChange={(event) => setDueDate(event.target.value)} />
        </label>
        <div className="capture-buttons">
          <button type="button" onClick={() => setOpen(false)}>
            取消
          </button>
          <button type="button" disabled={!text.trim()} onClick={() => void submit()}>
            添加
          </button>
        </div>
      </div>
    </div>
  ) : (
    <button
      id="task-capture"
      className="face-capture-trigger"
      type="button"
      disabled={disabled}
      onClick={() => setOpen(true)}
    >
      <span className="capture-plus">+</span>
      <span>
        <strong>添加一个待办</strong>
        <small>写下下一件要做的事</small>
      </span>
    </button>
  );
}

function RecordCapture({
  disabled,
  onCreate,
}: {
  disabled: boolean;
  onCreate(input: CreateStickyCardInput): Promise<boolean>;
}) {
  const [open, setOpen] = useState(false);
  const [hasText, setHasText] = useState(false);
  const textRef = useRef<HTMLTextAreaElement>(null);
  async function submit() {
    const text = textRef.current?.value ?? "";
    const input = parseCapture(text, "note", null);
    if (input && !disabled && (await onCreate(input))) {
      setHasText(false);
      setOpen(false);
    }
  }
  return open ? (
    <div id="note-capture" className="record-create-sheet">
      <textarea
        ref={textRef}
        autoFocus
        rows={8}
        maxLength={100000}
        aria-label="新建长记录"
        placeholder="第一行会成为列表标题。继续往下写，正文可以很长…"
        onChange={(event) => {
          const nextHasText = Boolean(event.target.value.trim());
          if (nextHasText !== hasText) setHasText(nextHasText);
        }}
        onKeyDown={(event) => {
          if (event.key === "Escape") setOpen(false);
          if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
            event.preventDefault();
            void submit();
          }
        }}
      />
      <div>
        <small>纯文本 · Ctrl/⌘ + Enter 保存</small>
        <span>
          <button type="button" onClick={() => setOpen(false)}>
            取消
          </button>
          <button type="button" disabled={!hasText} onClick={() => void submit()}>
            保存记录
          </button>
        </span>
      </div>
    </div>
  ) : (
    <button
      id="note-capture"
      className="face-capture-trigger"
      type="button"
      disabled={disabled}
      onClick={() => setOpen(true)}
    >
      <span className="capture-plus">+</span>
      <span>
        <strong>新建记录</strong>
        <small>在这张纸上写一篇长记录</small>
      </span>
    </button>
  );
}

function RecordList({
  records,
  disabled,
  onOpen,
}: {
  records: StickyCard[];
  disabled: boolean;
  onOpen(card: StickyCard): void;
}) {
  return (
    <div className="record-list" aria-label="我的记录">
      {records.length === 0 ? (
        <p className="face-empty">还没有记录。让第一段文字从这里开始。</p>
      ) : null}
      {records.map((record) => (
        <button type="button" key={record.id} disabled={disabled} onClick={() => onOpen(record)}>
          <strong>{recordDisplayTitle(record.text)}</strong>
          <time dateTime={record.updatedAt}>
            {new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric" }).format(
              new Date(record.updatedAt),
            )}{" "}
            · 最近修改
          </time>
          <span>{recordExcerpt(record.text)}</span>
        </button>
      ))}
    </div>
  );
}

function RecordEditor({
  record,
  disabled,
  onClose,
  onSave,
  onDelete,
  onExport,
  onCollapse,
  onDirtyChange,
}: {
  record: StickyCard;
  disabled: boolean;
  onClose(): void;
  onSave(id: string, text: string): Promise<boolean>;
  onDelete(id: string): Promise<void>;
  onExport(id: string): Promise<boolean>;
  onCollapse(): void;
  onDirtyChange(dirty: boolean): void;
}) {
  const draftRef = useRef<HTMLTextAreaElement>(null);
  const countRef = useRef<HTMLSpanElement>(null);
  const [saved, setSaved] = useState(true);
  async function save() {
    const draft = draftRef.current?.value ?? "";
    if (draft.trim() && (await onSave(record.id, draft))) {
      setSaved(true);
      onDirtyChange(false);
      return true;
    }
    return false;
  }
  async function saveAndCollapse() {
    if ((saved || (await save())) && draftRef.current?.value.trim()) onCollapse();
  }
  return (
    <section
      className="record-editor"
      aria-label={`编辑记录：${recordDisplayTitle(record.text)}`}
      aria-busy={disabled}
      data-dirty={!saved}
    >
      <header>
        <button type="button" onClick={onClose}>
          ← 记录
        </button>
        <span>{saved ? "已保存" : "有未保存修改"}</span>
      </header>
      <textarea
        ref={draftRef}
        defaultValue={record.text}
        maxLength={100000}
        aria-label="记录正文"
        onChange={(event) => {
          if (saved) {
            setSaved(false);
            onDirtyChange(true);
          }
          if (countRef.current)
            countRef.current.textContent = event.target.value.length.toLocaleString();
        }}
        onKeyDown={(event) => {
          if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
            event.preventDefault();
            void save();
          }
        }}
      />
      <footer>
        <span>
          <span ref={countRef}>{record.text.length.toLocaleString()}</span> 字符 · 纯文本
        </span>
        <div>
          <button type="button" disabled={disabled} onClick={() => void onExport(record.id)}>
            导出 .md
          </button>
          <button
            className="record-delete"
            type="button"
            disabled={disabled}
            onClick={() => void onDelete(record.id).then(onClose)}
          >
            删除
          </button>
          <button
            className="record-save"
            type="button"
            disabled={disabled || saved}
            onClick={() => void save()}
          >
            保存
          </button>
          <button
            className="record-collapse"
            type="button"
            disabled={disabled}
            onClick={() => void saveAndCollapse()}
          >
            {saved ? "收起" : "保存并收起"}
          </button>
        </div>
      </footer>
    </section>
  );
}

function SortableTask({
  card,
  disabled,
  onTaskCompleted,
  onTaskDueDate,
  onDelete,
}: {
  card: StickyCard;
  disabled: boolean;
  onTaskCompleted(id: string, completed: boolean): Promise<void>;
  onTaskDueDate(id: string, dueDate: string | null): Promise<void>;
  onDelete(id: string): Promise<void>;
}) {
  const [actionsOpen, setActionsOpen] = useState(false);
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: card.id,
    disabled,
  });
  const style: CSSProperties = { transform: CSS.Transform.toString(transform), transition };
  return (
    <article
      ref={setNodeRef}
      style={style}
      className="sticky-card-row"
      data-completed={card.completed}
      data-dragging={isDragging}
    >
      <button
        className="drag-handle"
        type="button"
        disabled={disabled}
        aria-label={`拖动排序：${card.text}`}
        {...attributes}
        {...listeners}
      >
        ⠿
      </button>
      <input
        className="task-checkbox"
        type="checkbox"
        checked={card.completed}
        disabled={disabled}
        aria-label={`完成待办：${card.text}`}
        onChange={(event) => void onTaskCompleted(card.id, event.target.checked)}
      />
      <div className="card-body">
        <span className="task-text">{card.text}</span>
        {card.dueDate ? (
          <time className="due-label" dateTime={card.dueDate}>
            {formatLocalDueDate(card.dueDate)}
          </time>
        ) : null}
      </div>
      <div className="card-actions">
        <button
          className="card-menu-button"
          type="button"
          aria-expanded={actionsOpen}
          onClick={() => setActionsOpen((value) => !value)}
        >
          •••
        </button>
        {actionsOpen ? (
          <div className="card-action-panel">
            <label>
              <span>完成日期</span>
              <input
                type="date"
                value={card.dueDate ?? ""}
                onChange={(event) => void onTaskDueDate(card.id, event.target.value || null)}
              />
            </label>
            <button className="delete-button" type="button" onClick={() => void onDelete(card.id)}>
              删除
            </button>
          </div>
        ) : null}
      </div>
    </article>
  );
}

function consumeTodoWheel(element: HTMLDivElement, event: globalThis.WheelEvent) {
  const canMoveUp = element.scrollTop > 0;
  const canMoveDown = element.scrollTop + element.clientHeight < element.scrollHeight - 1;
  if ((event.deltaY < 0 && canMoveUp) || (event.deltaY > 0 && canMoveDown)) {
    element.scrollTop += event.deltaY;
    event.preventDefault();
    event.stopPropagation();
    return;
  }
  const reader = document.querySelector<HTMLElement>(".reader-scroll-viewport");
  if (!reader || event.deltaY === 0) return;
  reader.scrollTop += event.deltaY;
  event.preventDefault();
}

type StickySurfaceProps = {
  mode: StickyMode;
  position: StickyPosition;
  pinned?: boolean;
  label: string;
  onActivate(): void;
  onPositionChange(position: StickyPosition): void;
  children: ReactNode;
};

function StickySurface({
  mode,
  position,
  pinned = false,
  label,
  onActivate,
  onPositionChange,
  children,
}: StickySurfaceProps) {
  const drag = useRef<{
    id: number;
    startX: number;
    startY: number;
    left: number;
    top: number;
    moved: boolean;
  } | null>(null);
  const [live, setLive] = useState<{ left: number; top: number } | null>(null);
  const [snapHint, setSnapHint] = useState<StickyPosition["snap"]>(null);
  const surfaceRef = useRef<HTMLDivElement>(null);

  const style: CSSProperties = pinned
    ? {}
    : live
      ? { left: live.left, top: live.top, transform: "none" }
      : position.snap
        ? {
            left: position.snap.endsWith("left") ? 12 : "auto",
            right: position.snap.endsWith("right") ? 12 : "auto",
            top: position.snap.startsWith("top") ? 76 : "auto",
            bottom: position.snap.startsWith("bottom") ? 12 : "auto",
            transform: "none",
          }
        : {
            left: `clamp(12px, calc(${position.xRatio * 100}% - var(--sticky-surface-half-width)), calc(100% - var(--sticky-surface-width) - 12px))`,
            top: `clamp(76px, calc(${position.yRatio * 100}% - var(--sticky-surface-half-height)), calc(100% - var(--sticky-surface-height) - 12px))`,
            transform: "none",
          };

  function pointerDown(event: PointerEvent<HTMLDivElement>) {
    if (pinned) return;
    if ((event.target as HTMLElement).closest("[data-no-drag]")) return;
    const surface = surfaceRef.current;
    if (!surface) return;
    const rect = surface.getBoundingClientRect();
    drag.current = {
      id: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      left: rect.left,
      top: rect.top,
      moved: false,
    };
    event.currentTarget.setPointerCapture?.(event.pointerId);
  }

  function pointerMove(event: PointerEvent<HTMLDivElement>) {
    const current = drag.current;
    const surface = surfaceRef.current;
    if (!current || current.id !== event.pointerId || !surface) return;
    const dx = event.clientX - current.startX;
    const dy = event.clientY - current.startY;
    if (Math.hypot(dx, dy) >= 5) current.moved = true;
    if (!current.moved) return;
    const board = surface.parentElement?.getBoundingClientRect();
    if (!board) return;
    const width = surface.offsetWidth;
    const height = surface.offsetHeight;
    const { left, top } = compactDragFrame({
      boardWidth: board.width,
      boardHeight: board.height,
      stickyWidth: width,
      stickyHeight: height,
      originLeft: current.left - board.left,
      originTop: current.top - board.top,
      deltaX: dx,
      deltaY: dy,
    });
    setLive({ left, top });
    const horizontal =
      left < SNAP_DISTANCE ? "left" : board.width - width - left < SNAP_DISTANCE ? "right" : null;
    const vertical =
      top - 76 < SNAP_DISTANCE
        ? "top"
        : board.height - height - top < SNAP_DISTANCE
          ? "bottom"
          : null;
    setSnapHint(
      horizontal && vertical ? (`${vertical}_${horizontal}` as StickyPosition["snap"]) : null,
    );
  }

  function pointerUp(event: PointerEvent<HTMLDivElement>) {
    const current = drag.current;
    const surface = surfaceRef.current;
    drag.current = null;
    if (!current || current.id !== event.pointerId || !surface) return;
    if (!current.moved) {
      onActivate();
      return;
    }
    const board = surface.parentElement?.getBoundingClientRect();
    if (!board || !live) return;
    const width = surface.offsetWidth;
    const height = surface.offsetHeight;
    onPositionChange(
      settleCompactPosition({
        boardWidth: board.width,
        boardHeight: board.height,
        stickyWidth: width,
        stickyHeight: height,
        left: live.left,
        top: live.top,
      }),
    );
    setLive(null);
    setSnapHint(null);
  }

  return (
    <div
      ref={surfaceRef}
      className="sticky-surface"
      data-mode={mode}
      data-pinned-to-shelf={pinned}
      data-snap-hint={snapHint ?? "free"}
      style={style}
      role="button"
      tabIndex={0}
      aria-label={label}
      onClick={pinned ? onActivate : undefined}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") onActivate();
      }}
      onPointerDown={pointerDown}
      onPointerMove={pointerMove}
      onPointerUp={pointerUp}
      onPointerCancel={() => {
        drag.current = null;
        setLive(null);
        setSnapHint(null);
      }}
    >
      <div className={mode === "mini" ? "mini-tab" : "sticky-preview"}>{children}</div>
    </div>
  );
}

function StickyPreview({
  cards,
  quote,
  loading,
  position,
  onExpand,
  onMinimize,
  onPositionChange,
}: {
  cards: StickyCard[];
  quote: string;
  loading: boolean;
  position: StickyPosition;
  onExpand(): void;
  onMinimize(): void;
  onPositionChange(position: StickyPosition): void;
}) {
  const openTasks = cards.filter((card) => card.kind === "task" && !card.completed);
  const todoViewportRef = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    const element = todoViewportRef.current;
    if (!element) return;
    const onWheel = (event: globalThis.WheelEvent) => consumeTodoWheel(element, event);
    element.addEventListener("wheel", onWheel, { passive: false });
    return () => element.removeEventListener("wheel", onWheel);
  }, [loading]);

  return (
    <StickySurface
      mode="compact"
      position={position}
      label="展开或拖动便利贴"
      onActivate={onExpand}
      onPositionChange={onPositionChange}
    >
      <span className="tape" aria-hidden="true" />
      <header>
        <span>TODAY</span>
        <span className="compact-header-actions">
          <small>
            {openTasks.length ? `${Math.min(3, openTasks.length)} / ${openTasks.length}` : "quiet"}
          </small>
          <button
            type="button"
            data-no-drag
            aria-label="缩成 Mini Tab"
            onClick={(event) => {
              event.stopPropagation();
              onMinimize();
            }}
          >
            −
          </button>
        </span>
      </header>
      {loading ? (
        <p className="preview-empty">正在翻开昨天留下的字迹…</p>
      ) : (
        <div
          ref={todoViewportRef}
          className="compact-todo-viewport"
          data-no-drag
          onClick={(event) => {
            event.stopPropagation();
            onExpand();
          }}
          aria-label="Compact Todo 列表"
        >
          {openTasks.length === 0 ? (
            <p className="preview-empty">今天没有未完成待办。</p>
          ) : (
            openTasks.map((task) => (
              <div className="preview-task" key={task.id}>
                <span className="preview-check" aria-hidden="true" />
                <span>{task.text}</span>
              </div>
            ))
          )}
        </div>
      )}
      <blockquote>{quote || "留一句愿意常常看到的话。"}</blockquote>
      <span className="preview-open">
        轻触展开 <span aria-hidden="true">→</span>
      </span>
    </StickySurface>
  );
}

function MiniStickyTab({
  openTaskCount,
  position,
  onRestore,
  onPositionChange,
}: {
  openTaskCount: number;
  position: StickyPosition;
  onRestore(): void;
  onPositionChange(position: StickyPosition): void;
}) {
  return (
    <StickySurface
      mode="mini"
      position={position}
      pinned
      label="恢复 Compact Sticky"
      onActivate={onRestore}
      onPositionChange={onPositionChange}
    >
      <span className="mini-fold" aria-hidden="true" />
      <span className="mini-label">TODAY</span>
      <strong>{openTaskCount}</strong>
      <span className="mini-dot" aria-hidden="true" />
    </StickySurface>
  );
}

function jumpToCapture(face: StickyFace) {
  const capture = document.getElementById(`${face}-capture`);
  capture?.scrollIntoView?.({ behavior: "smooth", block: "center" });
  if (capture instanceof HTMLButtonElement) capture.click();
}

export function StickyShell(props: StickyShellProps) {
  const {
    preferences,
    readerDocument,
    readerState,
    inboxItems,
    inboxUnreadCount,
    inboxState,
    inboxOpeningId,
    inboxOpenError,
    readingPlans,
    readingPlansState,
    readingPlansError,
    readingBusyPlanId,
    agentProposals,
    proposalBusyId,
    proposalErrorId,
    agentBridgeStatus,
    agentBridgeState,
    issuedAgentToken,
    agentTokenCopied,
    preferenceSaveState,
    cards,
    profile,
    stickyState,
    error,
    now = new Date(),
    onThemeChange,
    onAlwaysOnTopChange,
    onWindowPresetChange,
    onStickyPositionChange,
    onStickyModeChange,
    onReaderFontSizeChange,
    onReaderLineSpacingChange,
    onReaderContentVisibilityChange,
    onRetryReader,
    onRetryInbox,
    onOpenDelivery,
    onCopyReaderSelection,
    onCaptureReaderSelection,
    onCreateReadingSession,
    onRetryReadingPlans,
    onCreateReadingPlan,
    onGenerateReadingDelivery,
    onSetReadingPlanStatus,
    onAcceptProposal,
    onRejectProposal,
    onAgentBridgeEnabledChange,
    onGenerateAgentToken,
    onCopyAgentToken,
    onRetryAgentBridge,
    onCreate,
    onUpdateText,
    onTaskCompleted,
    onTaskDueDate,
    onDelete,
    onReorder,
    onUpdateQuote,
    onExportRecord,
    onRetry,
    onDismissError,
  } = props;
  const [expanded, setExpanded] = useState(false);
  const [readerSurfaceMode, setReaderSurfaceMode] = useState<ReaderSurfaceMode>("reader");
  const readerScrollTopRef = useRef(0);
  const [face, setFace] = useState<StickyFace>("note");
  const [selectedRecord, setSelectedRecord] = useState<StickyCard | null>(null);
  const [recordDirty, setRecordDirty] = useState(false);
  const [pendingCapture, setPendingCapture] = useState<StickyFace | null>(null);
  const [turnSerial, setTurnSerial] = useState(0);
  const [turnDirection, setTurnDirection] = useState<"forward" | "backward">("forward");
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );
  const records = cards.filter((card) => card.kind === "note");
  const tasks = cards.filter((card) => card.kind === "task");
  const openTaskCount = tasks.filter((card) => !card.completed).length;
  const busy = stickyState === "loading" || stickyState === "saving";
  const date = new Intl.DateTimeFormat(undefined, {
    weekday: "long",
    month: "short",
    day: "numeric",
  }).format(now);

  useEffect(() => {
    if (!pendingCapture || selectedRecord) return;
    const frame = requestAnimationFrame(() => {
      jumpToCapture(pendingCapture);
      setPendingCapture(null);
    });
    return () => cancelAnimationFrame(frame);
  }, [pendingCapture, selectedRecord]);

  function switchFace(nextFace: StickyFace) {
    if (nextFace === face) return;
    setSelectedRecord(null);
    setRecordDirty(false);
    setTurnDirection(nextFace === "task" ? "forward" : "backward");
    setFace(nextFace);
    setTurnSerial((value) => value + 1);
    playPageTurnSound(nextFace === "task" ? "note-to-todo" : "todo-to-note");
  }

  function openCapture() {
    if (!selectedRecord) {
      jumpToCapture(face);
      return;
    }
    setSelectedRecord(null);
    setRecordDirty(false);
    setPendingCapture(face);
  }

  function taskDragEnd(event: DragEndEvent) {
    if (!event.over || event.active.id === event.over.id) return;
    void onReorder(
      reorderCardIdsWithinKind(cards, "task", String(event.active.id), String(event.over.id)),
    );
  }

  function changeStickyMode(nextMode: StickyMode) {
    onStickyModeChange(nextMode);
  }

  async function openDelivery(id: string) {
    if (!(await onOpenDelivery(id))) return;
    readerScrollTopRef.current = 0;
    setReaderSurfaceMode("reader");
  }

  return (
    <main className="desk-board" aria-label="Agent Desk Sticky Home">
      <ReaderCanvas
        skin={preferences.readerSkin}
        fontSize={preferences.readerFontSize}
        lineSpacing={preferences.readerLineSpacing}
        windowPreset={preferences.windowPreset}
        document={readerDocument}
        state={readerState}
        surfaceMode={readerSurfaceMode}
        readerScrollTop={readerScrollTopRef.current}
        inboxItems={inboxItems}
        inboxState={inboxState}
        inboxOpeningId={inboxOpeningId}
        inboxOpenError={inboxOpenError}
        readingPlans={readingPlans}
        readingPlansState={readingPlansState}
        readingPlansError={readingPlansError}
        readingBusyPlanId={readingBusyPlanId}
        agentProposals={agentProposals}
        proposalBusyId={proposalBusyId}
        proposalErrorId={proposalErrorId}
        contentVisible={preferences.readerContentVisible}
        onRetry={onRetryReader}
        onRetryInbox={onRetryInbox}
        onOpenDelivery={(id) => void openDelivery(id)}
        onReaderScrollPositionChange={(scrollTop) => {
          readerScrollTopRef.current = scrollTop;
        }}
        onContentVisibilityChange={onReaderContentVisibilityChange}
        onCopy={onCopyReaderSelection}
        onCaptureSelection={onCaptureReaderSelection}
        onCreateReadingSession={onCreateReadingSession}
        onRetryReadingPlans={onRetryReadingPlans}
        onCreateReadingPlan={onCreateReadingPlan}
        onGenerateReadingDelivery={async (id) => {
          const generated = await onGenerateReadingDelivery(id);
          if (generated) setReaderSurfaceMode("inbox");
          return generated;
        }}
        onSetReadingPlanStatus={onSetReadingPlanStatus}
        onAcceptProposal={(id) => void onAcceptProposal(id)}
        onRejectProposal={(id) => void onRejectProposal(id)}
      />
      <header className="board-header">
        <div>
          <p className="board-eyebrow">Agent Desk</p>
          <h1>{date}</h1>
        </div>
        <div className="board-actions">
          <span className="persistence-dot" data-state={stickyState} />
          <button
            className="inbox-nav-button"
            type="button"
            aria-label={readerSurfaceMode === "plans" ? "返回阅读" : "打开阅读计划"}
            onClick={() => {
              if (readerSurfaceMode === "plans") {
                setReaderSurfaceMode("reader");
              } else {
                onRetryReadingPlans();
                setReaderSurfaceMode("plans");
              }
            }}
          >
            {readerSurfaceMode === "plans" ? "阅读" : "计划"}
          </button>
          <button
            className="inbox-nav-button"
            type="button"
            aria-label={
              readerSurfaceMode === "inbox"
                ? "返回阅读"
                : inboxUnreadCount > 0
                  ? `打开收件箱，${inboxUnreadCount} 件未打开`
                  : "打开收件箱"
            }
            onClick={() => {
              if (readerSurfaceMode !== "inbox") {
                onRetryInbox();
                setReaderSurfaceMode("inbox");
              } else {
                setReaderSurfaceMode("reader");
              }
            }}
          >
            {readerSurfaceMode === "inbox"
              ? "阅读"
              : inboxUnreadCount > 0
                ? `收件 ${inboxUnreadCount}`
                : "收件"}
          </button>
          <SettingsMenu
            preferences={preferences}
            saveState={preferenceSaveState}
            agentBridgeStatus={agentBridgeStatus}
            agentBridgeState={agentBridgeState}
            issuedAgentToken={issuedAgentToken}
            agentTokenCopied={agentTokenCopied}
            onThemeChange={onThemeChange}
            onAlwaysOnTopChange={onAlwaysOnTopChange}
            onWindowPresetChange={onWindowPresetChange}
            onReaderFontSizeChange={onReaderFontSizeChange}
            onReaderLineSpacingChange={onReaderLineSpacingChange}
            onAgentBridgeEnabledChange={(enabled) => void onAgentBridgeEnabledChange(enabled)}
            onGenerateAgentToken={() => void onGenerateAgentToken()}
            onCopyAgentToken={() => void onCopyAgentToken()}
            onRetryAgentBridge={() => void onRetryAgentBridge()}
          />
        </div>
      </header>
      <div className="sticky-overlay-layer" data-reader-layer="2">
        {expanded ? (
          <section className="expanded-sticky" aria-label="展开的便利贴">
            <span className="tape expanded-tape" aria-hidden="true" />
            <header className="expanded-header">
              <div className="face-switcher" role="tablist" aria-label="便利贴正反面">
                {(["note", "task"] as const).map((candidate) => (
                  <button
                    key={candidate}
                    type="button"
                    role="tab"
                    aria-selected={face === candidate}
                    disabled={busy || (selectedRecord !== null && recordDirty)}
                    onClick={() => switchFace(candidate)}
                  >
                    {faceLabels[candidate]}
                    <span>{candidate === "note" ? records.length : openTaskCount}</span>
                  </button>
                ))}
              </div>
              <div className="expanded-actions">
                <button
                  className="new-entry-shortcut"
                  type="button"
                  disabled={busy || (selectedRecord !== null && recordDirty)}
                  onClick={openCapture}
                >
                  + {face === "note" ? "新记录" : "新待办"}
                </button>
                <button
                  className="collapse-button"
                  type="button"
                  disabled={busy || (selectedRecord !== null && recordDirty)}
                  aria-label="收起便利贴"
                  onClick={() => {
                    setSelectedRecord(null);
                    setRecordDirty(false);
                    setExpanded(false);
                  }}
                >
                  ↙
                </button>
              </div>
            </header>
            <div className="face-scroll" data-editor={selectedRecord ? "record" : "none"}>
              <div
                key={`${face}-${turnSerial}`}
                className="sticky-face"
                data-face={face}
                data-turn={turnDirection}
                role="tabpanel"
              >
                {face === "note" ? (
                  selectedRecord ? (
                    <RecordEditor
                      record={cards.find((card) => card.id === selectedRecord.id) ?? selectedRecord}
                      disabled={busy}
                      onClose={() => {
                        setSelectedRecord(null);
                        setRecordDirty(false);
                      }}
                      onSave={onUpdateText}
                      onDelete={onDelete}
                      onExport={onExportRecord}
                      onCollapse={() => {
                        setSelectedRecord(null);
                        setRecordDirty(false);
                        setExpanded(false);
                      }}
                      onDirtyChange={setRecordDirty}
                    />
                  ) : (
                    <>
                      <QuoteEditor profile={profile} disabled={busy} onUpdate={onUpdateQuote} />
                      <div className="face-heading">
                        <p>我的记录</p>
                        <span>{records.length} 篇</span>
                      </div>
                      <RecordList
                        records={records}
                        disabled={busy}
                        onOpen={(record) => {
                          setRecordDirty(false);
                          setSelectedRecord(record);
                        }}
                      />
                      <RecordCapture disabled={busy} onCreate={onCreate} />
                    </>
                  )
                ) : (
                  <>
                    <div className="face-heading">
                      <p>接下来要做的事</p>
                      <span>{openTaskCount} 件未完成</span>
                    </div>
                    <DndContext
                      sensors={sensors}
                      collisionDetection={closestCenter}
                      onDragEnd={taskDragEnd}
                    >
                      <SortableContext
                        items={tasks.map((card) => card.id)}
                        strategy={verticalListSortingStrategy}
                      >
                        <div className="sticky-list">
                          {tasks.length === 0 ? (
                            <p className="face-empty">今天还没有待办。先写下一件小事。</p>
                          ) : null}
                          {tasks.map((card) => (
                            <SortableTask
                              key={card.id}
                              card={card}
                              disabled={busy}
                              onTaskCompleted={onTaskCompleted}
                              onTaskDueDate={onTaskDueDate}
                              onDelete={onDelete}
                            />
                          ))}
                        </div>
                      </SortableContext>
                    </DndContext>
                    <TaskCapture disabled={busy} onCreate={onCreate} />
                  </>
                )}
                {!selectedRecord ? <p className="paper-end-mark">✦</p> : null}
              </div>
            </div>
            {error ? (
              <div className="mutation-error" role="alert">
                <span>{error}</span>
                <div>
                  {stickyState === "error" && cards.length === 0 ? (
                    <button type="button" onClick={() => void onRetry()}>
                      重试
                    </button>
                  ) : null}
                  <button type="button" aria-label="关闭错误提示" onClick={onDismissError}>
                    ×
                  </button>
                </div>
              </div>
            ) : null}
          </section>
        ) : preferences.stickyMode === "mini" ? (
          <MiniStickyTab
            openTaskCount={openTaskCount}
            position={preferences.stickyPosition}
            onRestore={() => changeStickyMode("compact")}
            onPositionChange={onStickyPositionChange}
          />
        ) : (
          <StickyPreview
            cards={cards}
            quote={profile.quoteText}
            loading={stickyState === "loading"}
            position={preferences.stickyPosition}
            onExpand={() => setExpanded(true)}
            onMinimize={() => changeStickyMode("mini")}
            onPositionChange={onStickyPositionChange}
          />
        )}
      </div>
    </main>
  );
}
