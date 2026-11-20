import { invoke } from "@tauri-apps/api/core";

export type CodexPairingStatus = {
  phase: "disabled" | "disconnected" | "awaiting_test" | "connected" | "needs_authorization";
  executable: string | null;
  arguments: string[];
};

export type TodoGrantStatus = "disabled" | "disconnected" | "connected" | "needs_authorization";

export const tauriCodexPairing = {
  status: () => invoke<CodexPairingStatus>("get_codex_pairing"),
  authorize: () => invoke<CodexPairingStatus>("pair_codex"),
  revoke: () => invoke<CodexPairingStatus>("revoke_codex"),
  todoStatus: () => invoke<TodoGrantStatus>("get_codex_todo_grant"),
  authorizeTodo: () => invoke<TodoGrantStatus>("pair_codex_todo"),
  revokeTodo: () => invoke<TodoGrantStatus>("revoke_codex_todo"),
};
