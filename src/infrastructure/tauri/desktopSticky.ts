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
  openReader() {
    return invoke<Preferences>("open_reader_workspace");
  },
  setDesktopMode(stickyMode) {
    return invoke<Preferences>("set_desktop_sticky_mode", { stickyMode });
  },
  setReaderMode(stickyMode) {
    return invoke<Preferences>("set_reader_sticky_mode", { stickyMode });
  },
  leaveReader(stickyMode) {
    return invoke<Preferences>("leave_reader_to_desktop", { stickyMode });
  },
  startDragging() {
    return invoke("start_desktop_sticky_drag");
  },
  dockIfNearRightEdge() {
    return invoke("dock_desktop_sticky_if_near_right_edge");
  },
  showContextMenu() {
    return invoke("show_desktop_sticky_menu");
  },
  setEdgeDockExpanded(expanded) {
    return invoke("set_edge_dock_expanded", { expanded });
  },
};
