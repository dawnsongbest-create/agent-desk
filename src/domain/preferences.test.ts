import { describe, expect, it } from "vitest";
import { settleCompactPosition } from "./preferences";

const base = { boardWidth: 420, boardHeight: 594, stickyWidth: 218, stickyHeight: 176 };

describe("Compact Sticky position", () => {
  it("preserves a normalized free position", () => {
    expect(settleCompactPosition({ ...base, left: 100, top: 210 })).toEqual({
      xRatio: 209 / 420,
      yRatio: 298 / 594,
      snap: null,
    });
  });

  it.each([
    ["top_left", 4, 70],
    ["top_right", 200, 70],
    ["bottom_left", 4, 410],
    ["bottom_right", 200, 410],
  ] as const)("snaps to %s without leaving the board", (snap, left, top) => {
    const position = settleCompactPosition({ ...base, left, top });
    expect(position.snap).toBe(snap);
    expect(position.xRatio).toBeGreaterThan(0);
    expect(position.xRatio).toBeLessThan(1);
    expect(position.yRatio).toBeGreaterThan(0);
    expect(position.yRatio).toBeLessThan(1);
  });

  it("clamps an off-screen free request before normalizing it", () => {
    const position = settleCompactPosition({ ...base, left: 900, top: 900 }, 0);
    expect(position).toEqual({
      xRatio: 299 / 420,
      yRatio: 494 / 594,
      snap: null,
    });
  });
});
