import type { DesktopStickyKind, Preferences } from "../../domain/preferences";

export interface DesktopStickyWindowPort {
  enter(): Promise<Preferences>;
  open(editKind?: DesktopStickyKind): Promise<Preferences>;
  startDragging(): Promise<void>;
  showContextMenu(): Promise<void>;
}
