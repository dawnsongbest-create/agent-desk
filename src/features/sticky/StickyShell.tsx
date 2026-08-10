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
import { useEffect, useRef, useState, type CSSProperties, type KeyboardEvent } from "react";
import type { Preferences, ThemeMode } from "../../domain/preferences";
import { themeModes } from "../../domain/preferences";
import {
  formatLocalDueDate,
  parseCapture,
  reorderCardIds,
  type CreateStickyCardInput,
  type StickyCard,
  type StickyCardKind,
} from "../../domain/sticky";
import type { StickyLoadState } from "./useStickyCards";

type PreferenceSaveState = "loading" | "idle" | "saving" | "saved" | "error";

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

  return (
    <div className="settings-anchor">
      <button
        className="icon-button settings-button"
        type="button"
        aria-label="便利贴设置"
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
      >
        ···
      </button>
      {open ? (
        <div className="settings-panel" aria-label="便利贴设置面板">
          <span className="settings-label">外观</span>
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
          <button
            className="pin-button"
            type="button"
            disabled={disabled}
            aria-pressed={preferences.alwaysOnTop}
            onClick={() => onAlwaysOnTopChange(!preferences.alwaysOnTop)}
          >
            {preferences.alwaysOnTop ? "✓ 已置顶" : "置顶窗口"}
          </button>
        </div>
      ) : null}
    </div>
  );
}

function CaptureComposer({
  disabled,
  onCreate,
}: {
  disabled: boolean;
  onCreate(input: CreateStickyCardInput): Promise<boolean>;
}) {
  const [open, setOpen] = useState(false);
  const [kind, setKind] = useState<StickyCardKind>("note");
  const [text, setText] = useState("");
  const [dueDate, setDueDate] = useState("");
  const inputRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (open) inputRef.current?.focus();
  }, [open]);

  async function submit() {
    const input = parseCapture(text, kind, dueDate || null);
    if (!input || disabled) return;
    if (await onCreate(input)) {
      setText("");
      setDueDate("");
      setKind("note");
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
        className="capture-trigger"
        type="button"
        disabled={disabled}
        onClick={() => setOpen(true)}
      >
        <span aria-hidden="true">＋</span> 记点什么
      </button>
    );
  }

  return (
    <div className="capture-composer">
      <div className="capture-kind" role="group" aria-label="内容类型">
        <button type="button" aria-pressed={kind === "note"} onClick={() => setKind("note")}>
          随手记
        </button>
        <button type="button" aria-pressed={kind === "task"} onClick={() => setKind("task")}>
          任务
        </button>
      </div>
      <textarea
        ref={inputRef}
        value={text}
        rows={kind === "note" ? 3 : 2}
        maxLength={4000}
        placeholder={kind === "note" ? "写下此刻想到的事…" : "要完成什么？"}
        aria-label={kind === "note" ? "新随手记" : "新任务"}
        onChange={(event) => setText(event.target.value)}
        onKeyDown={handleKeyDown}
      />
      <div className="capture-actions">
        {kind === "task" ? (
          <label className="capture-due">
            <span>日期</span>
            <input
              type="date"
              value={dueDate}
              aria-label="新任务日期"
              onChange={(event) => setDueDate(event.target.value)}
            />
          </label>
        ) : (
          <span className="keyboard-hint">Ctrl/⌘ + Enter 保存</span>
        )}
        <button
          className="capture-submit"
          type="button"
          disabled={disabled || !text.trim()}
          onClick={() => void submit()}
        >
          记下
        </button>
      </div>
    </div>
  );
}

function SortableStickyCard({
  card,
  disabled,
  onUpdateText,
  onTaskCompleted,
  onTaskDueDate,
  onDelete,
}: {
  card: StickyCard;
  disabled: boolean;
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
    >
      <button
        className="drag-handle"
        type="button"
        disabled={disabled}
        aria-label={`重新排序：${card.text}`}
        {...attributes}
        {...listeners}
      >
        ⠿
      </button>

      {card.kind === "task" ? (
        <input
          className="task-checkbox"
          type="checkbox"
          checked={card.completed}
          disabled={disabled}
          aria-label={card.completed ? `恢复任务：${card.text}` : `完成任务：${card.text}`}
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
              aria-label="编辑随手记"
              onChange={(event) => setDraft(event.target.value)}
              onKeyDown={editKeyDown}
              onBlur={() => void commitEdit()}
            />
          ) : (
            <input
              className="inline-editor task-editor"
              value={draft}
              autoFocus
              aria-label="编辑任务"
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
          className="icon-button card-menu-button"
          type="button"
          disabled={disabled}
          aria-label={`更多操作：${card.text}`}
          aria-expanded={actionsOpen}
          onClick={() => setActionsOpen((value) => !value)}
        >
          ···
        </button>
        {actionsOpen ? (
          <div className="card-action-panel" aria-label={`${card.text} 操作`}>
            {card.kind === "task" ? (
              <label>
                <span>日期</span>
                <input
                  type="date"
                  value={card.dueDate ?? ""}
                  aria-label={`任务日期：${card.text}`}
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
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );
  const date = new Intl.DateTimeFormat(undefined, {
    weekday: "long",
    month: "short",
    day: "numeric",
  }).format(now);
  const busy = stickyState === "loading" || stickyState === "saving";

  function dragEnd(event: DragEndEvent) {
    if (!event.over || event.active.id === event.over.id) return;
    const orderedIds = reorderCardIds(cards, String(event.active.id), String(event.over.id));
    void onReorder(orderedIds);
  }

  return (
    <main className="sticky-shell" aria-label="Agent Desk Sticky Home">
      <header className="sticky-header">
        <div>
          <p className="eyebrow">Agent Desk</p>
          <h1 className="sticky-date">{date}</h1>
        </div>
        <div className="header-actions">
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

      <section className="sticky-content" aria-labelledby="today-heading">
        <h2 id="today-heading">今天</h2>
        {stickyState === "loading" ? (
          <p className="quiet-state" role="status">
            正在打开便利贴…
          </p>
        ) : (
          <DndContext
            sensors={sensors}
            collisionDetection={closestCenter}
            onDragEnd={dragEnd}
            accessibility={{
              screenReaderInstructions: {
                draggable:
                  "按空格拿起内容，使用方向键移动，再按空格放下。按 Escape 取消，不会保存排序。",
              },
            }}
          >
            <SortableContext
              items={cards.map((card) => card.id)}
              strategy={verticalListSortingStrategy}
            >
              <div className="sticky-list">
                {cards.map((card) => (
                  <SortableStickyCard
                    key={card.id}
                    card={card}
                    disabled={busy}
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
      </section>

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

      <footer className="capture-footer">
        <CaptureComposer disabled={busy} onCreate={onCreate} />
      </footer>
    </main>
  );
}
