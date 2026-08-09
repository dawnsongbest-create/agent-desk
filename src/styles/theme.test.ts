import { afterEach, describe, expect, it } from "vitest";
import { applyTheme } from "./theme";

afterEach(() => {
  delete document.documentElement.dataset.theme;
});

describe("applyTheme", () => {
  it("uses the OS theme by removing the explicit override", () => {
    document.documentElement.dataset.theme = "dark";
    applyTheme("system");
    expect(document.documentElement.dataset.theme).toBeUndefined();
  });

  it("sets an explicit theme", () => {
    applyTheme("light");
    expect(document.documentElement.dataset.theme).toBe("light");
  });
});
