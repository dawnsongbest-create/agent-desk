import type { DesktopStickyKind, Preferences } from "../../domain/preferences";

export interface DesktopStickyWindowPort {
  enter(): Promise<Preferences>;
  open(editKind?: DesktopStickyKind): Promise<Preferences>;
  openReader(): Promise<Preferences>;
  startDragging(): Promise<void>;
  dockIfNearRightEdge(): Promise<void>;
  showContextMenu(): Promise<void>;
  setEdgeDockExpanded(expanded: boolean): Promise<void>;
}
