import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";

const phases = {
  disconnected: "未连接",
  awaiting_first_call: "已授权，等待首次调用",
  connected: "已连接",
  partial: "部分权限已关闭",
  revoked: "已撤销",
  error: "连接异常",
} as const;

type Scopes = { reading: boolean; todo: boolean };
type Status = Scopes & { phase: keyof typeof phases; lastUsedAt: string | null };
const noScopes: Scopes = { reading: false, todo: false };

export function ChatGptPairingSettings() {
  const [status, setStatus] = useState<Status>();
  const [draft, setDraft] = useState<Scopes>();
  const [help, setHelp] = useState(false);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState("");
  const locked = useRef(false);
  const active = Boolean(status?.reading || status?.todo);

  useEffect(() => {
    let disposed = false;
    let timer: ReturnType<typeof setTimeout>;
    async function poll() {
      if (!locked.current) {
        try {
          const result = await invoke<Status>("get_chatgpt_status");
          if (!disposed && !locked.current) setStatus(result);
        } catch {
          if (!disposed && !locked.current) setStatus(undefined);
        }
      }
      if (!disposed) timer = setTimeout(() => void poll(), active ? 3500 : 10000);
    }
    void poll();
    return () => {
      disposed = true;
      clearTimeout(timer);
    };
  }, [active]);

  async function run(operation: () => Promise<void>) {
    if (locked.current) return;
    locked.current = true;
    setWorking(true);
    setError("");
    try {
      await operation();
    } catch (cause) {
      setError(typeof cause === "string" ? cause : "操作未完成，请检查应用和系统凭据库后重试。");
    } finally {
      locked.current = false;
      setWorking(false);
    }
  }

  function configure(scopes: Scopes, rotate = false) {
    return run(async () => {
      const updated = await invoke<Status>("configure_chatgpt", { ...scopes, rotate });
      setStatus(updated);
      setDraft(undefined);
      setHelp(updated.reading || updated.todo);
    });
  }

  function exportPackage() {
    return run(async () => {
      if (await invoke<boolean>("export_chatgpt_package")) setHelp(true);
    });
  }

  const lastUsed = status?.lastUsedAt
    ? new Date(
        status.lastUsedAt.includes(" ")
          ? status.lastUsedAt.replace(" ", "T") + "Z"
          : status.lastUsedAt,
      )
    : undefined;

  return (
    <section className="agent-bridge-settings" aria-label="ChatGPT">
      <strong>
        <span aria-hidden="true">◎ </span>ChatGPT
      </strong>
      <p role="status">{working ? "正在配置" : status ? phases[status.phase] : "状态暂不可用"}</p>
      <small>
        权限：阅读投递{status?.reading ? "已开启" : "未开启"} · 待办建议
        {status?.todo ? "已开启" : "未开启"}
      </small>
      {lastUsed && !Number.isNaN(lastUsed.getTime()) ? (
        <small>
          最近有效调用：<time dateTime={lastUsed.toISOString()}>{lastUsed.toLocaleString()}</time>
        </small>
      ) : null}
      <p>
        只发送你要求发送的内容。此连接不能读取 Agent Desk
        的私人记录，也不能直接更改待办。可随时断开。
      </p>
      {error ? <p role="alert">{error}</p> : null}
      {draft ? (
        <fieldset disabled={working}>
          <legend>选择本机授权</legend>
          <label>
            <input
              type="checkbox"
              checked={draft.reading}
              onChange={(event) => setDraft({ ...draft, reading: event.target.checked })}
            />{" "}
            阅读投递：允许发送文章、报告和总结
          </label>
          <br />
          <label>
            <input
              type="checkbox"
              checked={draft.todo}
              onChange={(event) => setDraft({ ...draft, todo: event.target.checked })}
            />{" "}
            待办建议：等待你确认后才创建待办
          </label>
          <div className="agent-bridge-actions">
            <button
              type="button"
              disabled={!active && !draft.reading && !draft.todo}
              onClick={() => void configure(draft)}
            >
              确认并保存授权
            </button>
            <button type="button" onClick={() => setDraft(undefined)}>
              取消
            </button>
          </div>
        </fieldset>
      ) : (
        <div className="agent-bridge-actions">
          <button
            type="button"
            disabled={working}
            onClick={() =>
              setDraft(active && status ? { reading: status.reading, todo: status.todo } : noScopes)
            }
          >
            {active ? "管理权限" : "开始连接"}
          </button>
          {active && status ? (
            <>
              <button type="button" disabled={working} onClick={() => void exportPackage()}>
                保存安装包
              </button>
              <button
                type="button"
                disabled={working}
                onClick={() => void configure({ reading: status.reading, todo: status.todo }, true)}
              >
                轮换凭据
              </button>
              <button type="button" disabled={working} onClick={() => void configure(noScopes)}>
                断开 ChatGPT
              </button>
            </>
          ) : null}
        </div>
      )}
      {active && help ? (
        <div>
          <p>
            保存 ZIP，再到 ChatGPT Desktop → Work → Local → Plugins，从本地安装此
            ZIP。无需复制凭据。
          </p>
          <p>
            打开一个新的 Local 会话，用 @ 选择「Agent Desk for
            ChatGPT」，请它发送一篇测试文章或待办建议（按已开启权限）。收到首张回执后刷新连接状态。
          </p>
        </div>
      ) : null}
    </section>
  );
}
