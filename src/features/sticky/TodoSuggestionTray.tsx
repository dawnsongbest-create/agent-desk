import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useId, useRef, useState } from "react";
import {
  TODO_SUGGESTIONS_CHANGED,
  type TodoSuggestionBatch,
  type TodoSuggestionsPort,
} from "../../application/ports/todoSuggestions";
import { formatLocalDueDate } from "../../domain/sticky";

const priorityLabels = { high: "高", medium: "中", low: "低" } as const;
const sourceLabels = { codex: "Codex", dumate: "百度搭子" } as const;

export function TodoSuggestionTray({
  port,
  onAccepted,
  onLocate,
}: {
  port: TodoSuggestionsPort;
  onAccepted(): void;
  onLocate(id: string): void;
}) {
  const panelId = useId();
  const version = useRef(0);
  const inFlight = useRef(false);
  const [batches, setBatches] = useState<TodoSuggestionBatch[]>([]);
  const [open, setOpen] = useState(false);
  const [activeId, setActiveId] = useState<string>();
  const [busy, setBusy] = useState(false);
  const [readError, setReadError] = useState(false);
  const [failed, setFailed] = useState<{
    batch: TodoSuggestionBatch;
    action: "accept" | "reject";
  }>();
  const [receipt, setReceipt] = useState<string[]>();
  const active = batches.find((batch) => batch.id === activeId) ?? failed?.batch;

  const reload = useCallback(async () => {
    const attempt = ++version.current;
    try {
      const pending = await port.listPending();
      if (attempt !== version.current) return;
      setBatches(pending);
      setReadError(false);
    } catch {
      if (attempt === version.current) setReadError(true);
    }
  }, [port]);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    void reload();
    void listen(TODO_SUGGESTIONS_CHANGED, () => {
      if (!cancelled) void reload();
    })
      .then((dispose) => {
        if (cancelled) dispose();
        else {
          unlisten = dispose;
          void reload(); // Catch a submission during listener registration.
        }
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
      version.current += 1;
      unlisten?.();
    };
  }, [reload]);

  async function resolve(batch: TodoSuggestionBatch, action: "accept" | "reject") {
    if (inFlight.current) return;
    inFlight.current = true;
    setBusy(true);
    setFailed(undefined);
    try {
      if (action === "accept") {
        const result = await port.accept(batch.id);
        setReceipt(result.todoIds);
        onAccepted();
      } else {
        await port.reject(batch.id);
      }
      setBatches((current) => current.filter((item) => item.id !== batch.id));
      setActiveId(undefined);
      setOpen(false);
    } catch {
      // Keep the exact batch available for an explicit retry, even if the response was lost.
      setFailed({ batch, action });
      if (action === "accept") onAccepted();
    } finally {
      void reload();
      setBusy(false);
      inFlight.current = false;
    }
  }

  if (batches.length === 0 && !readError && !receipt && !failed && !busy) return null;

  return (
    <aside className="todo-suggestion-tray" aria-label="待确认的待办建议">
      {batches.length ? (
        <button
          type="button"
          className="todo-suggestion-link"
          aria-expanded={open}
          aria-controls={open ? panelId : undefined}
          disabled={busy}
          onClick={() => {
            setOpen(!open);
            setActiveId(!open && batches.length === 1 ? batches[0].id : undefined);
            setFailed(undefined);
            if (!open) void reload();
          }}
        >
          待确认建议 · {batches.length}
        </button>
      ) : null}
      {readError ? (
        <p role="alert">
          无法读取待办建议。{" "}
          <button type="button" className="todo-suggestion-link" onClick={() => void reload()}>
            重试
          </button>
        </p>
      ) : null}
      {open ? (
        <div id={panelId} className="todo-suggestion-panel">
          {!active ? (
            <ul className="todo-suggestion-batches">
              {batches.map((batch) => (
                <li key={batch.id}>
                  <button
                    type="button"
                    className="todo-suggestion-link"
                    disabled={busy}
                    onClick={() => setActiveId(batch.id)}
                  >
                    {sourceLabels[batch.source]} · {batch.items.length} 项 ·{" "}
                    {batch.items[0]?.text.slice(0, 36)}
                  </button>
                </li>
              ))}
            </ul>
          ) : (
            <>
              <p>
                {sourceLabels[active.source]} 建议 · {active.items.length} 项 · 确认后才加入待办
              </p>
              <ol className="todo-suggestion-items">
                {active.items.map((item, index) => (
                  <li key={index}>
                    <small>
                      {priorityLabels[item.priority]}重要性
                      {item.due_date ? ` · ${formatLocalDueDate(item.due_date)}` : ""}
                    </small>
                    <p>{item.text}</p>
                    {item.reason ? <small>{item.reason}</small> : null}
                  </li>
                ))}
              </ol>
              {failed ? <p role="alert">未能确认操作结果。请重试原操作，或暂不处理。</p> : null}
              <div className="todo-suggestion-actions">
                <button
                  type="button"
                  disabled={busy || failed?.action === "reject"}
                  onClick={() => void resolve(active, "accept")}
                >
                  确认加入
                </button>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => {
                    setOpen(false);
                    setActiveId(undefined);
                    setFailed(undefined);
                  }}
                >
                  暂不处理
                </button>
                <button
                  type="button"
                  disabled={busy || failed?.action === "accept"}
                  onClick={() => void resolve(active, "reject")}
                >
                  拒绝
                </button>
              </div>
            </>
          )}
        </div>
      ) : null}
      {receipt ? (
        <p className="todo-suggestion-receipt">
          <span role="status">已加入 {receipt.length} 项</span>
          {receipt[0] ? (
            <button
              type="button"
              className="todo-suggestion-link"
              onClick={() => onLocate(receipt[0])}
            >
              查看刚加入的待办
            </button>
          ) : null}
        </p>
      ) : null}
    </aside>
  );
}
