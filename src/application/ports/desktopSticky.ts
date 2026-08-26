import type { DesktopStickyKind, Preferences, StickyMode } from "../../domain/preferences";

export interface DesktopStickyWindowPort {
  enter(): Promise<Preferences>;
  open(editKind?: DesktopStickyKind): Promise<Preferences>;
  openReader(): Promise<Preferences>;
  setDesktopMode(mode: StickyMode): Promise<Preferences>;
  setReaderMode(mode: StickyMode): Promise<Preferences>;
  leaveReader(mode: StickyMode): Promise<Preferences>;
  startDragging(): Promise<void>;
  dockIfNearRightEdge(): Promise<void>;
  showContextMenu(): Promise<void>;
  setEdgeDockExpanded(expanded: boolean): Promise<void>;
}
