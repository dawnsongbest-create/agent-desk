import { invoke } from "@tauri-apps/api/core";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ChatGptPairingSettings } from "./ChatGptPairingSettings";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

describe("ChatGPT pairing", () => {
  it("does not grant by default and does not claim connection on authorization or export", async () => {
    let readingGranted = false;
    vi.mocked(invoke).mockImplementation(async (method, args) => {
      if (method === "export_chatgpt_package") return true;
      if (method === "configure_chatgpt") {
        readingGranted = Boolean((args as { reading?: boolean })?.reading);
      }
      return {
        reading: readingGranted,
        todo: false,
        phase: readingGranted ? "awaiting_first_call" : "disconnected",
        lastUsedAt: null,
      };
    });
    const user = userEvent.setup();
    render(<ChatGptPairingSettings />);
    await screen.findByText("未连接");
    await user.click(screen.getByRole("button", { name: "开始连接" }));
    const [reading, todo] = screen.getAllByRole("checkbox");
    expect(reading).not.toBeChecked();
    expect(todo).not.toBeChecked();
    expect(screen.getByRole("button", { name: "确认并保存授权" })).toBeDisabled();
    await user.click(reading);
    await user.click(screen.getByRole("button", { name: "确认并保存授权" }));
    expect(invoke).toHaveBeenCalledWith("configure_chatgpt", {
      reading: true,
      todo: false,
      rotate: false,
    });
    await screen.findByText("已授权，等待首次调用");
    await user.click(screen.getByRole("button", { name: "保存安装包" }));
    expect(invoke).toHaveBeenCalledWith("export_chatgpt_package");
    expect(screen.queryByText("已连接", { exact: true })).not.toBeInTheDocument();
  });
});
