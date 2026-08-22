import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { AgentBridgeSettings } from "./AgentBridgeSettings";

const status = {
  version: "v1" as const,
  enabled: true,
  running: true,
  bindAddress: "127.0.0.1" as const,
  port: 47321,
  endpoint: "http://127.0.0.1:47321/api/v1",
  connection: {
    id: "local-agent-bridge",
    name: "Local Agent Bridge",
    status: "active" as const,
    hasToken: true,
    createdAt: "2026-08-22T00:00:00Z",
    updatedAt: "2026-08-22T00:00:00Z",
    lastUsedAt: null,
  },
  lastError: null,
};

describe("AgentBridgeSettings", () => {
  it("shows localhost provenance and the newly issued token only when provided", async () => {
    const user = userEvent.setup();
    const copy = vi.fn();
    const { rerender } = render(
      <AgentBridgeSettings
        status={status}
        state="ready"
        issuedToken={null}
        copied={false}
        onEnabledChange={vi.fn()}
        onGenerateToken={vi.fn()}
        onCopyToken={copy}
        onRetry={vi.fn()}
      />,
    );
    expect(screen.getByText("仅本机运行中")).toBeInTheDocument();
    expect(screen.getByText(status.endpoint)).toBeInTheDocument();
    expect(screen.queryByLabelText("新 Agent Token")).not.toBeInTheDocument();

    rerender(
      <AgentBridgeSettings
        status={status}
        state="ready"
        issuedToken="adk_one_time_token"
        copied={false}
        onEnabledChange={vi.fn()}
        onGenerateToken={vi.fn()}
        onCopyToken={copy}
        onRetry={vi.fn()}
      />,
    );
    expect(screen.getByText("adk_one_time_token")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "复制 Token" }));
    expect(copy).toHaveBeenCalledOnce();
  });
});
