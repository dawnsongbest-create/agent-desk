import type { AgentBridgeStatus, GenerateAgentTokenResult } from "../../domain/agentConnection";

export interface AgentConnectionPort {
  getStatus(): Promise<AgentBridgeStatus>;
  start(): Promise<AgentBridgeStatus>;
  stop(): Promise<AgentBridgeStatus>;
  generateToken(): Promise<GenerateAgentTokenResult>;
  copyToken(token: string): Promise<void>;
}
