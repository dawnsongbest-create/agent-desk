import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import {
  tauriCodexPairing,
  type CodexPairingStatus,
  type TodoGrantStatus,
} from "../../infrastructure/tauri/codexPairing";

const phaseLabels: Record<CodexPairingStatus["phase"], string> = {
  disabled: "Codex 投递已禁用",
  disconnected: "未连接",
  awaiting_test: "已授权，等待首次测试发送",
  connected: "已连接",
  needs_authorization: "需要重新授权",
};

export function CodexPairingSettings({ bridgeRunning }: { bridgeRunning: boolean }) {
  const [status, setStatus] = useState<CodexPairingStatus | null>(null);
  const [todoGrant, setTodoGrant] = useState<TodoGrantStatus | null>(null);
  const [showSetup, setShowSetup] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    const [reading, todo] = await Promise.allSettled([
      tauriCodexPairing.status(),
      tauriCodexPairing.todoStatus(),
    ]);
    if (reading.status === "fulfilled") setStatus(reading.value);
    if (todo.status === "fulfilled") setTodoGrant(todo.value);
    if (reading.status === "rejected" || todo.status === "rejected") {
      setError("无法读取 Codex 连接状态。请重试。");
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    void refresh();
    void listen("agent-desk://inbox-changed", () => {
      if (!cancelled) void refresh();
    })
      .then((dispose) => {
        if (cancelled) dispose();
        else unlisten = dispose;
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [refresh, bridgeRunning]);

  async function change(action: "authorize" | "revoke" | "authorizeTodo" | "revokeTodo") {
    setBusy(true);
    setError(null);
    try {
      if (action === "authorizeTodo" || action === "revokeTodo") {
        setTodoGrant(await tauriCodexPairing[action]());
      } else {
        setStatus(await tauriCodexPairing[action]());
        if (action === "revoke") {
          setShowSetup(false);
          setTodoGrant(await tauriCodexPairing.todoStatus());
        }
      }
    } catch (failure) {
      setError(typeof failure === "string" ? failure : "无法更新 Codex 连接。请重试。");
      await refresh();
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="agent-bridge-settings" aria-label="Codex 连接与权限">
      <p role="status">
        Codex：{status ? phaseLabels[status.phase] : "状态待检查"}
        {!bridgeRunning ? " · Bridge 已停止" : ""}
      </p>
      {error ? <p role="alert">{error}</p> : null}
      <div className="agent-bridge-actions">
        <button
          type="button"
          disabled={busy || status?.phase === "disabled"}
          onClick={() => {
            setShowSetup((previous) => !previous);
            setError(null);
            void refresh();
          }}
        >
          连接 Codex
        </button>
        {(status && (status.phase !== "disconnected" || showSetup)) ||
        todoGrant === "connected" ||
        todoGrant === "needs_authorization" ? (
          <button type="button" disabled={busy} onClick={() => void change("revoke")}>
            断开 Codex
          </button>
        ) : null}
      </div>
      {showSetup && status ? (
        <div>
          {status.executable ? (
            <>
              <p>
                在本机 Codex 的「MCP servers」手动新增 STDIO 服务，名称可用
                agent-desk-reading。不要覆盖已有配置。
              </p>
              <p>命令（填写路径，不经过 Shell）：</p>
              <code className="agent-bridge-endpoint">{status.executable}</code>
              <p>按顺序各填一个参数：{status.arguments.join(" · ")}</p>
              <p>
                保存并重启 Codex，确认可见 send_reading 和
                suggest_todos。待办建议需单独授权，无需粘贴 Token。
              </p>
              <button
                type="button"
                disabled={busy || !bridgeRunning || status.phase === "disabled"}
                onClick={() => void change("authorize")}
              >
                已注册，授权 Codex
              </button>
              {status.phase === "awaiting_test" ? (
                <p>
                  在 Codex 输入：「把一篇标题为 Agent Desk 连接测试的合成报告发送到 Agent
                  Desk」。收到首次已提交回执后，本页显示已连接。
                </p>
              ) : null}
            </>
          ) : (
            <p role="alert">找不到本地连接器，请从安装后的 Agent Desk 重试。</p>
          )}
          <p>彻底移除时，先断开授权，再在 Codex 设置中只移除你刚才添加的服务。</p>
        </div>
      ) : null}
      {todoGrant && todoGrant !== "disabled" ? (
        <div aria-label="Codex 待办建议权限">
          <p>
            待办建议：
            {todoGrant === "connected"
              ? "已授权"
              : todoGrant === "needs_authorization"
                ? "需要重新授权"
                : "未授权"}
          </p>
          <p>Codex 只能提交建议，不能直接创建、修改、删除或完成你的待办。</p>
          <div className="agent-bridge-actions">
            {todoGrant !== "connected" ? (
              <button
                type="button"
                disabled={busy || !bridgeRunning}
                onClick={() => void change("authorizeTodo")}
              >
                允许提交待确认的待办建议
              </button>
            ) : null}
            {todoGrant !== "disconnected" ? (
              <button type="button" disabled={busy} onClick={() => void change("revokeTodo")}>
                撤销待办建议权限
              </button>
            ) : null}
          </div>
        </div>
      ) : null}
    </div>
  );
}
