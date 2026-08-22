import { useCallback, useEffect, useState } from "react";
import type { AgentConnectionPort } from "../../application/ports/agentConnection";
import type { AgentBridgeStatus } from "../../domain/agentConnection";

export type AgentBridgeLoadState = "loading" | "ready" | "working" | "error";

export function useAgentBridge(port: AgentConnectionPort) {
  const [status, setStatus] = useState<AgentBridgeStatus | null>(null);
  const [state, setState] = useState<AgentBridgeLoadState>("loading");
  const [issuedToken, setIssuedToken] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const refresh = useCallback(async () => {
    setState("loading");
    try {
      setStatus(await port.getStatus());
      setState("ready");
    } catch {
      setState("error");
    }
  }, [port]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function setEnabled(enabled: boolean) {
    setState("working");
    try {
      setStatus(await (enabled ? port.start() : port.stop()));
      setState("ready");
    } catch {
      setState("error");
    }
  }

  async function generateToken() {
    setState("working");
    try {
      const result = await port.generateToken();
      setStatus(result.status);
      setIssuedToken(result.token);
      setCopied(false);
      setState("ready");
    } catch {
      setState("error");
    }
  }

  async function copyToken() {
    if (!issuedToken) return;
    try {
      await port.copyToken(issuedToken);
      setCopied(true);
    } catch {
      setCopied(false);
    }
  }

  return {
    status,
    state,
    issuedToken,
    copied,
    refresh,
    setEnabled,
    generateToken,
    copyToken,
  };
}
