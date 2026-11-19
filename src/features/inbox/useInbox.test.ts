import { listen } from "@tauri-apps/api/event";
import { act, renderHook, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import type { InboxDelivery } from "../../domain/delivery";
import { useInbox } from "./useInbox";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn().mockResolvedValue(vi.fn()) }));

it("refreshes list and unread count on native invalidation without opening an item", async () => {
  const now = "2026-11-19T00:00:00Z";
  const item: InboxDelivery = {
    delivery: {
      id: "d",
      documentId: "r",
      idempotencyKey: "isolate",
      deliveredAt: now,
      openedAt: null,
    },
    document: {
      id: "r",
      documentType: "report",
      title: "Synthetic report",
      subtitle: null,
      contentMarkdown: "# 完整 Markdown",
      sourceType: "agent",
      sourceLabel: "Codex",
      createdAt: now,
      updatedAt: now,
    },
  };
  const port = {
    ingest: vi.fn(),
    open: vi.fn(),
    listInbox: vi.fn<() => Promise<InboxDelivery[]>>().mockResolvedValue([]),
    getUnreadCount: vi.fn().mockResolvedValue(0),
  };
  const { result } = renderHook(() => useInbox(port, true));
  await waitFor(() => expect(result.current.state).toBe("ready"));
  port.listInbox.mockResolvedValue([item]);
  port.getUnreadCount.mockResolvedValue(1);
  const event = "agent-desk://inbox-changed";
  const listener = vi.mocked(listen).mock.calls.find(([name]) => name === event)?.[1];
  expect(listener).toBeDefined();
  await act(async () => listener?.({ event, id: 1, payload: null }));
  expect(result.current.items).toEqual([item]);
  expect(result.current.unreadCount).toBe(1);
  expect(port.open).not.toHaveBeenCalled();
});
