import { invoke } from "@tauri-apps/api/core";
import type { DesktopStickyWindowPort } from "../../application/ports/desktopSticky";
import type { Preferences } from "../../domain/preferences";

export const tauriDesktopSticky: DesktopStickyWindowPort = {
  enter() {
    return invoke<Preferences>("enter_desktop_sticky");
  },
  open(editKind) {
    return invoke<Preferences>("open_agent_desk_from_desktop", { editKind: editKind ?? null });
  },
  startDragging() {
    return invoke("start_desktop_sticky_drag");
  },
  showContextMenu() {
    return invoke("show_desktop_sticky_menu");
  },
};
