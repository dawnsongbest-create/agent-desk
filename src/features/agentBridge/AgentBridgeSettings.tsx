import type { AgentBridgeLoadState } from "./useAgentBridge";
import type { AgentBridgeStatus } from "../../domain/agentConnection";

type AgentBridgeSettingsProps = {
  status: AgentBridgeStatus | null;
  state: AgentBridgeLoadState;
  issuedToken: string | null;
  copied: boolean;
  onEnabledChange(enabled: boolean): void;
  onGenerateToken(): void;
  onCopyToken(): void;
  onRetry(): void;
};

export function AgentBridgeSettings({
  status,
  state,
  issuedToken,
  copied,
  onEnabledChange,
  onGenerateToken,
  onCopyToken,
  onRetry,
}: AgentBridgeSettingsProps) {
  const disabled = state === "loading" || state === "working";
  if (state === "error" && !status) {
    return (
      <div className="agent-bridge-settings" aria-label="本地 Agent Bridge">
        <p role="alert">无法读取本地连接状态。</p>
        <button type="button" onClick={onRetry}>
          重试
        </button>
      </div>
    );
  }

  return (
    <div className="agent-bridge-settings" aria-label="本地 Agent Bridge">
      <div className="agent-bridge-status-row">
        <span className="agent-bridge-status-dot" data-running={status?.running ?? false} />
        <span>{status?.running ? "仅本机运行中" : "已停止"}</span>
        <code>API v1</code>
      </div>
      <code className="agent-bridge-endpoint">
        {status?.endpoint ?? "http://127.0.0.1:47321/api/v1"}
      </code>
      {status?.lastError ? <p role="alert">{status.lastError}</p> : null}
      <div className="agent-bridge-actions">
        <button
          type="button"
          disabled={disabled}
          aria-pressed={status?.enabled ?? false}
          onClick={() => onEnabledChange(!(status?.enabled ?? false))}
        >
          {status?.enabled ? "停止 Bridge" : "启用 Bridge"}
        </button>
        <button type="button" disabled={disabled} onClick={onGenerateToken}>
          {status?.connection?.hasToken ? "轮换 Token" : "生成 Token"}
        </button>
      </div>
      <small>Token 仅保存在本机，Bridge 只监听 127.0.0.1。</small>
      {issuedToken ? (
        <div className="agent-token-once" aria-label="新 Agent Token">
          <small>仅显示这一次，请立即复制。再次生成会使旧 Token 失效。</small>
          <code>{issuedToken}</code>
          <button type="button" onClick={onCopyToken}>
            {copied ? "已复制" : "复制 Token"}
          </button>
        </div>
      ) : null}
    </div>
  );
}
