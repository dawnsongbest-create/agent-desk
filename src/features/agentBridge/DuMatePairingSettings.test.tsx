import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { DuMateStatus } from "../../infrastructure/tauri/dumatePairing";
import { DuMatePairingSettings } from "./DuMatePairingSettings";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const offline: DuMateStatus = {
  previewAvailable: false,
  bridgeRunning: false,
  endpoint: "http://127.0.0.1:49219/mcp",
  reading: false,
  todo: false,
  expiresAt: null,
};

beforeEach(() => vi.resetAllMocks());

it("does not offer credentials or a connector when the isolated preview is off", async () => {
  vi.mocked(invoke).mockResolvedValue(offline);
  render(<DuMatePairingSettings bridgeRunning={false} />);
  expect(await screen.findByText(/默认关闭/)).toBeVisible();
  expect(screen.queryByRole("button", { name: /复制 Authorization/ })).not.toBeInTheDocument();
});

it("saves only selected scopes and never puts the copied credential in the DOM", async () => {
  const user = userEvent.setup();
  const clipboard = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue();
  const sampleAuthorization = "Bearer synthetic-noncredential";
  let status: DuMateStatus = { ...offline, previewAvailable: true, bridgeRunning: true };
  vi.mocked(invoke).mockImplementation(async (command, args) => {
    if (command === "get_dumate_status") return status;
    if (command === "copy_dumate_authorization") return sampleAuthorization;
    if (command === "configure_dumate") {
      const reading = args !== undefined && "reading" in args && args.reading === true;
      const todo = args !== undefined && "todo" in args && args.todo === true;
      status = {
        ...status,
        reading,
        todo,
        expiresAt: reading || todo ? Math.floor(Date.now() / 1000) + 1800 : null,
      };
      return status;
    }
    throw new Error("unexpected command");
  });
  render(<DuMatePairingSettings bridgeRunning />);
  const panel = within(await screen.findByRole("region", { name: "百度搭子" }));
  expect(panel.getByText(offline.endpoint)).toBeVisible();
  expect(panel.getByText(/无法保证仅保存在本机/)).toBeVisible();
  await user.click(panel.getByRole("checkbox", { name: "阅读投递" }));
  await user.click(panel.getByRole("button", { name: "创建 30 分钟授权" }));
  expect(vi.mocked(invoke)).toHaveBeenCalledWith("configure_dumate", {
    reading: true,
    todo: false,
    rotate: false,
  });
  expect(await panel.findByText("阅读：已授权 · 待办建议：未授权")).toBeVisible();
  await user.click(panel.getByRole("button", { name: "复制 Authorization 的值" }));
  expect(clipboard).toHaveBeenCalledExactlyOnceWith(sampleAuthorization);
  expect(panel.queryByText(sampleAuthorization)).not.toBeInTheDocument();
  await user.click(panel.getByRole("button", { name: "撤销百度搭子" }));
  expect(await panel.findByText(/旧凭据无法投递/)).toBeVisible();
  expect(panel.getByRole("button", { name: "复制 Authorization 的值" })).toBeDisabled();
});
