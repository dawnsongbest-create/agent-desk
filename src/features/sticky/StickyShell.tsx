import {
  closestCenter,
  DndContext,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragOverEvent,
  type DragStartEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { useEffect, useRef, useState, type CSSProperties, type KeyboardEvent } from "react";
import type { Preferences, ThemeMode } from "../../domain/preferences";
import { themeModes } from "../../domain/preferences";
import {
  formatLocalDueDate,
  parseCapture,
  reorderCardIdsWithinKind,
  type CreateStickyCardInput,
  type StickyCard,
} from "../../domain/sticky";
import { playPageTurnSound } from "./pageTurnSound";
import type { StickyLoadState } from "./useStickyCards";

type PreferenceSaveState = "loading" | "idle" | "saving" | "saved" | "error";
type StickyFace = "note" | "task";

type StickyShellProps = {
  preferences: Preferences;
  preferenceSaveState: PreferenceSaveState;
  cards: StickyCard[];
  stickyState: StickyLoadState;
  error: string | null;
  now?: Date;
  onThemeChange(theme: ThemeMode): void;
  onAlwaysOnTopChange(alwaysOnTop: boolean): void;
  onCreate(input: CreateStickyCardInput): Promise<boolean>;
  onUpdateText(id: string, text: string): Promise<boolean>;
  onTaskCompleted(id: string, completed: boolean): Promise<void>;
  onTaskDueDate(id: string, dueDate: string | null): Promise<void>;
  onDelete(id: string): Promise<void>;
  onReorder(orderedIds: string[]): Promise<void>;
  onRetry(): Promise<void>;
  onDismissError(): void;
};

const themeLabels: Record<ThemeMode, string> = {
  system: "跟随系统",
  light: "浅色",
  dark: "深色",
};

const faceLabels: Record<StickyFace, string> = {
  note: "记录",
  task: "待办",
};

function SettingsMenu({
  preferences,
  saveState,
  onThemeChange,
  onAlwaysOnTopChange,
}: {
  preferences: Preferences;
  saveState: PreferenceSaveState;
  onThemeChange(theme: ThemeMode): void;
  onAlwaysOnTopChange(alwaysOnTop: boolean): void;
}) {
  const [open, setOpen] = useState(false);
  const disabled = saveState === "loading" || saveState === "saving";

  function chooseTheme(theme: ThemeMode) {
    onThemeChange(theme);
    setOpen(false);
  }

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
            <span>{saveState === "saving" ? "正在保存…" : "选择纸张明暗"}</span>
          </div>
          <div className="theme-switcher" role="group" aria-label="主题">
            {themeModes.map((theme) => (
              <button
                className="theme-button"
                type="button"
                key={theme}
                disabled={disabled}
                aria-pressed={preferences.theme === theme}
                onClick={() => chooseTheme(theme)}
              >
                {themeLabels[theme]}
              </button>
            ))}
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

function FaceCapture({
  kind,
  disabled,
  onCreate,
}: {
  kind: StickyFace;
  disabled: boolean;
  onCreate(input: CreateStickyCardInput): Promise<boolean>;
}) {
  const [open, setOpen] = useState(false);
  const [text, setText] = useState("");
  const [dueDate, setDueDate] = useState("");
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const captureId = `${kind}-capture`;

  useEffect(() => {
    if (open) inputRef.current?.focus();
  }, [open]);

  async function submit() {
    const input = parseCapture(text, kind, dueDate || null);
    if (!input || disabled) return;
    if (await onCreate(input)) {
      setText("");
      setDueDate("");
      setOpen(false);
    }
  }

  function handleKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key === "Escape") {
      setText("");
      setDueDate("");
      setOpen(false);
      return;
    }
    const noteSubmit = kind === "note" && (event.metaKey || event.ctrlKey) && event.key === "Enter";
    const taskSubmit = kind === "task" && event.key === "Enter" && !event.shiftKey;
    if (noteSubmit || taskSubmit) {
      event.preventDefault();
      void submit();
    }
  }

  if (!open) {
    return (
      <button
        id={captureId}
        className="face-capture-trigger"
        type="button"
        disabled={disabled}
        onClick={() => setOpen(true)}
      >
        <span className="capture-plus" aria-hidden="true">
          +
        </span>
        <span>
          <strong>{kind === "note" ? "继续写一条记录" : "添加一个待办"}</strong>
          <small>{kind === "note" ? "像在纸上接着往下写" : "写下下一件要做的事"}</small>
        </span>
      </button>
    );
  }

  return (
    <div id={captureId} className="face-capture-composer" data-kind={kind}>
      <textarea
        ref={inputRef}
        value={text}
        rows={kind === "note" ? 4 : 2}
        maxLength={4000}
        placeholder={kind === "note" ? "写下此刻想到的事…" : "要完成什么？"}
        aria-label={kind === "note" ? "新记录" : "新待办"}
        onChange={(event) => setText(event.target.value)}
        onKeyDown={handleKeyDown}
      />
      <div className="capture-actions">
        {kind === "task" ? (
          <label className="capture-due">
            <span>日期（可选）</span>
            <input
              type="date"
              value={dueDate}
              aria-label="新待办日期"
              onChange={(event) => setDueDate(event.target.value)}
            />
          </label>
        ) : (
          <span className="keyboard-hint">Ctrl/⌘ + Enter 保存 · 输入 [ ] 可快速建待办</span>
        )}
        <div className="capture-buttons">
          <button type="button" className="capture-cancel" onClick={() => setOpen(false)}>
            取消
          </button>
          <button
            className="capture-submit"
            type="button"
            disabled={disabled || !text.trim()}
            onClick={() => void submit()}
          >
            {kind === "note" ? "写下" : "添加"}
          </button>
        </div>
      </div>
    </div>
  );
}

function SortableStickyCard({
  card,
  disabled,
  over,
  onUpdateText,
  onTaskCompleted,
  onTaskDueDate,
  onDelete,
}: {
  card: StickyCard;
  disabled: boolean;
  over: boolean;
  onUpdateText(id: string, text: string): Promise<boolean>;
  onTaskCompleted(id: string, completed: boolean): Promise<void>;
  onTaskDueDate(id: string, dueDate: string | null): Promise<void>;
  onDelete(id: string): Promise<void>;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(card.text);
  const [actionsOpen, setActionsOpen] = useState(false);
  const cancelledEdit = useRef(false);
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: card.id,
    disabled,
  });
  const style: CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  useEffect(() => {
    if (!editing) setDraft(card.text);
  }, [card.text, editing]);

  async function commitEdit() {
    if (cancelledEdit.current) {
      cancelledEdit.current = false;
      setDraft(card.text);
      setEditing(false);
      return;
    }
    const normalized = draft.trim();
    if (!normalized || normalized === card.text) {
      setDraft(card.text);
      setEditing(false);
      return;
    }
    await onUpdateText(card.id, normalized);
    setEditing(false);
  }

  function editKeyDown(event: KeyboardEvent<HTMLInputElement | HTMLTextAreaElement>) {
    if (event.key === "Escape") {
      cancelledEdit.current = true;
      event.currentTarget.blur();
      return;
    }
    if (card.kind === "task" && event.key === "Enter") {
      event.preventDefault();
      event.currentTarget.blur();
      return;
    }
    if (card.kind === "note" && event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
      event.preventDefault();
      event.currentTarget.blur();
    }
  }

  return (
    <article
      ref={setNodeRef}
      style={style}
      className="sticky-card-row"
      data-kind={card.kind}
      data-completed={card.completed}
      data-dragging={isDragging}
      data-drop-target={over && !isDragging}
    >
      <button
        className="drag-handle"
        type="button"
        disabled={disabled}
        aria-label={`拖动排序：${card.text}`}
        {...attributes}
        {...listeners}
      >
        <span aria-hidden="true">⠿</span>
      </button>

      {card.kind === "task" ? (
        <input
          className="task-checkbox"
          type="checkbox"
          checked={card.completed}
          disabled={disabled}
          aria-label={card.completed ? `恢复待办：${card.text}` : `完成待办：${card.text}`}
          onChange={(event) => void onTaskCompleted(card.id, event.target.checked)}
        />
      ) : (
        <span className="note-mark" aria-hidden="true">
          —
        </span>
      )}

      <div className="card-body">
        {editing ? (
          card.kind === "note" ? (
            <textarea
              className="inline-editor note-editor"
              value={draft}
              rows={Math.max(2, draft.split("\n").length)}
              autoFocus
              aria-label="编辑记录"
              onChange={(event) => setDraft(event.target.value)}
              onKeyDown={editKeyDown}
              onBlur={() => void commitEdit()}
            />
          ) : (
            <input
              className="inline-editor task-editor"
              value={draft}
              autoFocus
              aria-label="编辑待办"
              onChange={(event) => setDraft(event.target.value)}
              onKeyDown={editKeyDown}
              onBlur={() => void commitEdit()}
            />
          )
        ) : (
          <button
            className="card-text-button"
            type="button"
            disabled={disabled}
            onClick={() => setEditing(true)}
          >
            {card.text}
          </button>
        )}
        {card.kind === "task" && card.dueDate ? (
          <time className="due-label" dateTime={card.dueDate}>
            {formatLocalDueDate(card.dueDate)}
          </time>
        ) : null}
      </div>

      <div className="card-actions">
        <button
          className="card-menu-button"
          type="button"
          disabled={disabled}
          aria-label={`更多操作：${card.text}`}
          aria-expanded={actionsOpen}
          onClick={() => setActionsOpen((value) => !value)}
        >
          •••
        </button>
        {actionsOpen ? (
          <div className="card-action-panel" aria-label={`${card.text} 操作`}>
            {card.kind === "task" ? (
              <label>
                <span>完成日期</span>
                <input
                  type="date"
                  value={card.dueDate ?? ""}
                  aria-label={`待办日期：${card.text}`}
                  onChange={(event) => void onTaskDueDate(card.id, event.target.value || null)}
                />
              </label>
            ) : null}
            <button className="delete-button" type="button" onClick={() => void onDelete(card.id)}>
              删除
            </button>
          </div>
        ) : null}
      </div>
    </article>
  );
}

function StickyPreview({
  cards,
  loading,
  onExpand,
}: {
  cards: StickyCard[];
  loading: boolean;
  onExpand(): void;
}) {
  const notes = cards.filter((card) => card.kind === "note");
  const tasks = cards.filter((card) => card.kind === "task");
  const openTasks = tasks.filter((card) => !card.completed);
  const previewTasks = openTasks.slice(0, 2);

  return (
    <button className="sticky-preview" type="button" onClick={onExpand} aria-label="展开便利贴">
      <span className="tape" aria-hidden="true" />
      <span className="preview-kicker">今天的纸条</span>
      {loading ? (
        <span className="preview-empty">正在翻开昨天留下的字迹…</span>
      ) : cards.length === 0 ? (
        <span className="preview-empty">
          纸上还很安静。
          <br />
          点一下，写下第一件事。
        </span>
      ) : (
        <>
          <span className="preview-summary">
            {notes.length} 条记录 · {openTasks.length} 件待办
          </span>
          <span className="preview-rule" />
          {previewTasks.map((task) => (
            <span className="preview-task" key={task.id}>
              <span aria-hidden="true">○</span> {task.text}
            </span>
          ))}
          {notes[0] ? <span className="preview-note">“{notes[0].text}”</span> : null}
        </>
      )}
      <span className="preview-open">
        轻触展开 <span aria-hidden="true">↗</span>
      </span>
    </button>
  );
}

function jumpToCapture(face: StickyFace) {
  const capture = document.getElementById(`${face}-capture`);
  capture?.scrollIntoView({ behavior: "smooth", block: "center" });
  if (capture instanceof HTMLButtonElement) capture.click();
}

export function StickyShell({
  preferences,
  preferenceSaveState,
  cards,
  stickyState,
  error,
  now = new Date(),
  onThemeChange,
  onAlwaysOnTopChange,
  onCreate,
  onUpdateText,
  onTaskCompleted,
  onTaskDueDate,
  onDelete,
  onReorder,
  onRetry,
  onDismissError,
}: StickyShellProps) {
  const [expanded, setExpanded] = useState(false);
  const [face, setFace] = useState<StickyFace>("note");
  const [turnSerial, setTurnSerial] = useState(0);
  const [turnDirection, setTurnDirection] = useState<"forward" | "backward">("forward");
  const [overId, setOverId] = useState<string | null>(null);
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );
  const date = new Intl.DateTimeFormat(undefined, {
    weekday: "long",
    month: "short",
    day: "numeric",
  }).format(now);
  const busy = stickyState === "loading" || stickyState === "saving";
  const visibleCards = cards.filter((card) => card.kind === face);
  const noteCount = cards.filter((card) => card.kind === "note").length;
  const openTaskCount = cards.filter((card) => card.kind === "task" && !card.completed).length;

  function switchFace(nextFace: StickyFace) {
    if (nextFace === face) return;
    setTurnDirection(nextFace === "task" ? "forward" : "backward");
    setFace(nextFace);
    setTurnSerial((value) => value + 1);
    playPageTurnSound(nextFace === "task" ? "note-to-todo" : "todo-to-note");
  }

  function dragStart(event: DragStartEvent) {
    setOverId(String(event.active.id));
  }

  function dragOver(event: DragOverEvent) {
    setOverId(event.over ? String(event.over.id) : null);
  }

  function dragEnd(event: DragEndEvent) {
    setOverId(null);
    if (!event.over || event.active.id === event.over.id) return;
    const orderedIds = reorderCardIdsWithinKind(
      cards,
      face,
      String(event.active.id),
      String(event.over.id),
    );
    void onReorder(orderedIds);
  }

  return (
    <main className="desk-board" aria-label="Agent Desk Sticky Home">
      <header className="board-header">
        <div>
          <p className="board-eyebrow">Agent Desk</p>
          <h1>{date}</h1>
        </div>
        <div className="board-actions">
          <span
            className="persistence-dot"
            data-state={stickyState}
            aria-label={stickyState === "saving" ? "正在保存" : "已存入本地"}
          />
          <SettingsMenu
            preferences={preferences}
            saveState={preferenceSaveState}
            onThemeChange={onThemeChange}
            onAlwaysOnTopChange={onAlwaysOnTopChange}
          />
        </div>
      </header>

      <div className="board-lines" aria-hidden="true" />
      <p className="board-caption">A quiet place beside your reading.</p>

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
                  aria-controls="sticky-face-panel"
                  onClick={() => switchFace(candidate)}
                >
                  {faceLabels[candidate]}
                  <span>{candidate === "note" ? noteCount : openTaskCount}</span>
                </button>
              ))}
            </div>
            <div className="expanded-actions">
              <button
                className="new-entry-shortcut"
                type="button"
                disabled={busy}
                onClick={() => jumpToCapture(face)}
              >
                + {face === "note" ? "新记录" : "新待办"}
              </button>
              <button
                className="collapse-button"
                type="button"
                aria-label="收起便利贴"
                onClick={() => setExpanded(false)}
              >
                ↙
              </button>
            </div>
          </header>

          <div className="face-scroll">
            <div
              key={`${face}-${turnSerial}`}
              id="sticky-face-panel"
              className="sticky-face"
              data-face={face}
              data-turn={turnDirection}
              role="tabpanel"
              aria-label={`${faceLabels[face]}面`}
            >
              <div className="face-heading">
                <p>{face === "note" ? "纸上的片刻" : "接下来要做的事"}</p>
                <span>
                  {face === "note" ? `${visibleCards.length} 条记录` : `${openTaskCount} 件未完成`}
                </span>
              </div>

              {stickyState === "loading" ? (
                <p className="quiet-state" role="status">
                  正在打开便利贴…
                </p>
              ) : (
                <DndContext
                  sensors={sensors}
                  collisionDetection={closestCenter}
                  onDragStart={dragStart}
                  onDragOver={dragOver}
                  onDragCancel={() => setOverId(null)}
                  onDragEnd={dragEnd}
                  accessibility={{
                    screenReaderInstructions: {
                      draggable:
                        "按空格拿起内容，使用方向键移动，再按空格放下。按 Escape 取消，不会保存排序。",
                    },
                  }}
                >
                  <SortableContext
                    items={visibleCards.map((card) => card.id)}
                    strategy={verticalListSortingStrategy}
                  >
                    <div className="sticky-list">
                      {visibleCards.length === 0 ? (
                        <p className="face-empty">
                          {face === "note"
                            ? "还没有记录。让第一句话从这里开始。"
                            : "今天还没有待办。先写下一件小事。"}
                        </p>
                      ) : null}
                      {visibleCards.map((card) => (
                        <SortableStickyCard
                          key={card.id}
                          card={card}
                          disabled={busy}
                          over={overId === card.id}
                          onUpdateText={onUpdateText}
                          onTaskCompleted={onTaskCompleted}
                          onTaskDueDate={onTaskDueDate}
                          onDelete={onDelete}
                        />
                      ))}
                    </div>
                  </SortableContext>
                </DndContext>
              )}

              <FaceCapture kind={face} disabled={busy} onCreate={onCreate} />
              <p className="paper-end-mark" aria-hidden="true">
                ✦
              </p>
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
      ) : (
        <StickyPreview
          cards={cards}
          loading={stickyState === "loading"}
          onExpand={() => setExpanded(true)}
        />
      )}
    </main>
  );
}
