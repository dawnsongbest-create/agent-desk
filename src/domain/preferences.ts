export const themeModes = ["system", "light", "dark"] as const;

export type ThemeMode = (typeof themeModes)[number];
export type WindowBehavior = "hide_to_tray";
export const windowPresets = ["sticky", "iphone5", "pocket", "book", "custom"] as const;
export type WindowPreset = (typeof windowPresets)[number];

export type StickyPosition = {
  xRatio: number;
  yRatio: number;
  snap: "top_left" | "top_right" | "bottom_left" | "bottom_right" | null;
};

export type Preferences = {
  schemaVersion: 1;
  theme: ThemeMode;
  alwaysOnTop: boolean;
  windowBehavior: WindowBehavior;
  windowPreset: WindowPreset;
  stickyPosition: StickyPosition;
};

export const defaultPreferences: Preferences = {
  schemaVersion: 1,
  theme: "system",
  alwaysOnTop: false,
  windowBehavior: "hide_to_tray",
  windowPreset: "sticky",
  stickyPosition: { xRatio: 0.5, yRatio: 0.28, snap: null },
};

export type CompactGeometry = {
  boardWidth: number;
  boardHeight: number;
  stickyWidth: number;
  stickyHeight: number;
  left: number;
  top: number;
};

export function settleCompactPosition(
  geometry: CompactGeometry,
  snapDistance = 28,
): StickyPosition {
  const { boardWidth, boardHeight, stickyWidth, stickyHeight } = geometry;
  const left = Math.max(12, Math.min(boardWidth - stickyWidth - 12, geometry.left));
  const top = Math.max(76, Math.min(boardHeight - stickyHeight - 12, geometry.top));
  const horizontal =
    left < snapDistance ? "left" : boardWidth - stickyWidth - left < snapDistance ? "right" : null;
  const vertical =
    top - 76 < snapDistance
      ? "top"
      : boardHeight - stickyHeight - top < snapDistance
        ? "bottom"
        : null;
  const snap =
    horizontal && vertical ? (`${vertical}_${horizontal}` as StickyPosition["snap"]) : null;
  const anchors = {
    top_left: { left: 12, top: 76 },
    top_right: { left: boardWidth - stickyWidth - 12, top: 76 },
    bottom_left: { left: 12, top: boardHeight - stickyHeight - 12 },
    bottom_right: { left: boardWidth - stickyWidth - 12, top: boardHeight - stickyHeight - 12 },
  };
  const settled = snap ? anchors[snap] : { left, top };
  return {
    xRatio: Math.max(0, Math.min(1, (settled.left + stickyWidth / 2) / boardWidth)),
    yRatio: Math.max(0, Math.min(1, (settled.top + stickyHeight / 2) / boardHeight)),
    snap,
  };
}
