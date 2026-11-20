import { invoke } from "@tauri-apps/api/core";

export type DuMateScopes = { reading: boolean; todo: boolean };
export type DuMateStatus = DuMateScopes & {
  previewAvailable: boolean;
  bridgeRunning: boolean;
  endpoint: string;
  expiresAt: number | null;
};

export const getDuMateStatus = () => invoke<DuMateStatus>("get_dumate_status");
export const configureDuMate = (scopes: DuMateScopes, rotate: boolean) =>
  invoke<DuMateStatus>("configure_dumate", { ...scopes, rotate });

export async function copyDuMateAuthorization() {
  // Windows writes to the clipboard natively. Other platforms use the
  // WebView clipboard from this explicit click; never cache or render it.
  const fallback = await invoke<string | null>("copy_dumate_authorization");
  if (typeof fallback === "string") {
    if (!navigator.clipboard) throw new Error("clipboard unavailable");
    await navigator.clipboard.writeText(fallback);
  }
}
