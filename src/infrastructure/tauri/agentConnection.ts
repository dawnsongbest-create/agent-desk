import { invoke } from "@tauri-apps/api/core";
import type { AgentConnectionPort } from "../../application/ports/agentConnection";
import type { AgentBridgeStatus, GenerateAgentTokenResult } from "../../domain/agentConnection";

export const tauriAgentConnection: AgentConnectionPort = {
  getStatus() {
    return invoke<AgentBridgeStatus>("get_agent_bridge_status");
  },
  start() {
    return invoke<AgentBridgeStatus>("start_agent_bridge");
  },
  stop() {
    return invoke<AgentBridgeStatus>("stop_agent_bridge");
  },
  generateToken() {
    return invoke<GenerateAgentTokenResult>("generate_agent_token");
  },
  async copyToken(token) {
    await navigator.clipboard.writeText(token);
  },
};
