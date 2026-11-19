import { invoke } from "@tauri-apps/api/core";

export type CodexPairingStatus = {
  phase: "disabled" | "disconnected" | "awaiting_test" | "connected" | "needs_authorization";
  executable: string | null;
  arguments: string[];
};

export const tauriCodexPairing = {
  status: () => invoke<CodexPairingStatus>("get_codex_pairing"),
  authorize: () => invoke<CodexPairingStatus>("pair_codex"),
  revoke: () => invoke<CodexPairingStatus>("revoke_codex"),
};
