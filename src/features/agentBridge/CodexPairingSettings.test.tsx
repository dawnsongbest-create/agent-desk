import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { tauriCodexPairing } from "../../infrastructure/tauri/codexPairing";
import { CodexPairingSettings } from "./CodexPairingSettings";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn().mockResolvedValue(vi.fn()) }));
vi.mock("../../infrastructure/tauri/codexPairing", () => ({
  tauriCodexPairing: { status: vi.fn(), authorize: vi.fn(), revoke: vi.fn() },
}));

it("waits for explicit authorization after showing the registration instructions", async () => {
  const user = userEvent.setup();
  const base = {
    executable: "agent-desk",
    arguments: ["--codex-mcp", "47321"],
  };
  vi.mocked(tauriCodexPairing.status).mockResolvedValue({ ...base, phase: "disconnected" });
  vi.mocked(tauriCodexPairing.authorize).mockResolvedValue({ ...base, phase: "awaiting_test" });
  vi.mocked(tauriCodexPairing.revoke).mockResolvedValue({ ...base, phase: "disconnected" });
  render(<CodexPairingSettings bridgeRunning />);
  expect(await screen.findByText("Codex：未连接")).toBeVisible();
  await user.click(screen.getByRole("button", { name: "连接 Codex" }));
  expect(screen.getByText("agent-desk")).toBeVisible();
  expect(tauriCodexPairing.authorize).not.toHaveBeenCalled();
  await user.click(screen.getByRole("button", { name: "已注册，授权 Codex" }));
  expect(await screen.findByText("Codex：已授权，等待首次测试发送")).toBeVisible();
  await user.click(screen.getByRole("button", { name: "断开 Codex" }));
  expect(await screen.findByText("Codex：未连接")).toBeVisible();
  expect(tauriCodexPairing.revoke).toHaveBeenCalledOnce();
});
