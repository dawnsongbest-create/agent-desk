import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { defaultPreferences } from "../../domain/preferences";
import { StickyShell } from "./StickyShell";

describe("StickyShell", () => {
  it("renders the minimal foundation surface without final capture controls", () => {
    render(
      <StickyShell
        preferences={defaultPreferences}
        saveState="idle"
        now={new Date("2026-08-10T00:00:00Z")}
        onThemeChange={vi.fn()}
        onAlwaysOnTopChange={vi.fn()}
      />,
    );

    expect(screen.getByRole("main", { name: "Agent Desk Sticky Home" })).toBeInTheDocument();
    expect(screen.getByText("M1-A")).toBeInTheDocument();
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
  });

  it("reports theme and always-on-top choices through callbacks", async () => {
    const user = userEvent.setup();
    const onThemeChange = vi.fn();
    const onAlwaysOnTopChange = vi.fn();

    render(
      <StickyShell
        preferences={defaultPreferences}
        saveState="saved"
        onThemeChange={onThemeChange}
        onAlwaysOnTopChange={onAlwaysOnTopChange}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Dark" }));
    await user.click(screen.getByRole("button", { name: "Pin window" }));

    expect(onThemeChange).toHaveBeenCalledWith("dark");
    expect(onAlwaysOnTopChange).toHaveBeenCalledWith(true);
  });

  it("blocks preference changes until stored preferences finish loading", () => {
    render(
      <StickyShell
        preferences={defaultPreferences}
        saveState="loading"
        onThemeChange={vi.fn()}
        onAlwaysOnTopChange={vi.fn()}
      />,
    );

    expect(screen.getByRole("button", { name: "Dark" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Pin window" })).toBeDisabled();
    expect(screen.getByText("Loading preferences…")).toBeInTheDocument();
  });
});
