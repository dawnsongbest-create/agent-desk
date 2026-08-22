export type AgentConnection = {
  id: string;
  name: string;
  status: "active" | "inactive";
  hasToken: boolean;
  createdAt: string;
  updatedAt: string;
  lastUsedAt: string | null;
};

export type AgentBridgeStatus = {
  version: "v1";
  enabled: boolean;
  running: boolean;
  bindAddress: "127.0.0.1";
  port: number;
  endpoint: string;
  connection: AgentConnection | null;
  lastError: string | null;
};

export type GenerateAgentTokenResult = {
  token: string;
  status: AgentBridgeStatus;
};
