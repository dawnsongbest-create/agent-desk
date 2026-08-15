import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { defaultPreferences } from "./domain/preferences";

const mocks = vi.hoisted(() => ({
  loadPreferences: vi.fn(),
  savePreferences: vi.fn(),
  applyWindowPreset: vi.fn(),
  listCards: vi.fn(),
  getProfile: vi.fn(),
}));

vi.mock("./infrastructure/tauri/preferences", () => ({
  tauriPreferences: {
    load: mocks.loadPreferences,
    save: mocks.savePreferences,
    applyWindowPreset: mocks.applyWindowPreset,
  },
}));

vi.mock("./infrastructure/tauri/sticky", () => ({
  tauriStickyCards: {
    list: mocks.listCards,
    getProfile: mocks.getProfile,
    create: vi.fn(),
    updateText: vi.fn(),
    setTaskCompleted: vi.fn(),
    setTaskDueDate: vi.fn(),
    delete: vi.fn(),
    reorder: vi.fn(),
    updateQuote: vi.fn(),
    exportRecord: vi.fn(),
  },
}));

import App from "./App";

describe("App Reader preferences", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.loadPreferences.mockResolvedValue(defaultPreferences);
    mocks.savePreferences.mockImplementation(async (preferences) => preferences);
    mocks.listCards.mockResolvedValue([]);
    mocks.getProfile.mockResolvedValue({ quoteText: "", updatedAt: "" });
  });

  it("persists font size and line spacing through the Preference Port", async () => {
    const user = userEvent.setup();
    render(<App />);

    const reader = await screen.findByRole("region", { name: "Reader Canvas" });
    expect(reader).toHaveAttribute("data-font-size", "standard");
    expect(reader).toHaveAttribute("data-line-spacing", "standard");

    await user.click(screen.getByRole("button", { name: "外观与窗口设置" }));
    await user.click(within(screen.getByRole("group", { name: "Reader 字号" })).getByText("大"));

    await waitFor(() => {
      expect(mocks.savePreferences).toHaveBeenCalledWith({
        ...defaultPreferences,
        readerFontSize: "large",
      });
      expect(reader).toHaveAttribute("data-font-size", "large");
    });

    await user.click(within(screen.getByRole("group", { name: "Reader 行距" })).getByText("宽松"));

    await waitFor(() => {
      expect(mocks.savePreferences).toHaveBeenLastCalledWith({
        ...defaultPreferences,
        readerFontSize: "large",
        readerLineSpacing: "relaxed",
      });
      expect(reader).toHaveAttribute("data-line-spacing", "relaxed");
    });
  });

  it("restores stored Reader preferences when the application starts", async () => {
    mocks.loadPreferences.mockResolvedValue({
      ...defaultPreferences,
      readerFontSize: "small",
      readerLineSpacing: "compact",
    });

    render(<App />);

    const reader = await screen.findByRole("region", { name: "Reader Canvas" });
    await waitFor(() => {
      expect(reader).toHaveAttribute("data-font-size", "small");
      expect(reader).toHaveAttribute("data-line-spacing", "compact");
    });
  });
});
