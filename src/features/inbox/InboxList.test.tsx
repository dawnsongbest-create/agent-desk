import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { InboxDelivery } from "../../domain/delivery";
import { InboxList } from "./InboxList";

function item(index: number, unread: boolean): InboxDelivery {
  return {
    delivery: {
      id: `delivery-${index}`,
      documentId: `reader-${index}`,
      idempotencyKey: `key-${index}`,
      deliveredAt: `2026-08-${15 - index}T08:00:00.000Z`,
      openedAt: unread ? null : "2026-08-15T09:00:00.000Z",
    },
    document: {
      id: `reader-${index}`,
      documentType: index % 2 === 0 ? "brief" : "article",
      title: `送达内容 ${index}`,
      subtitle: null,
      contentMarkdown: `正文 ${index}`,
      sourceType: "agent",
      sourceLabel: "Daily Brief",
      createdAt: "2026-08-15T08:00:00.000Z",
      updatedAt: "2026-08-15T08:00:00.000Z",
    },
  };
}

describe("InboxList", () => {
  it("renders the quiet exact empty state", () => {
    render(
      <InboxList
        items={[]}
        state="ready"
        openingId={null}
        openError={null}
        onRetry={vi.fn()}
        onOpen={vi.fn()}
      />,
    );
    expect(screen.getByText("暂时没有新内容。")).toBeVisible();
  });

  it("renders three unread and two opened deliveries with semantic labels", () => {
    render(
      <InboxList
        items={[item(1, true), item(2, true), item(3, true), item(4, false), item(5, false)]}
        state="ready"
        openingId={null}
        openError={null}
        onRetry={vi.fn()}
        onOpen={vi.fn()}
      />,
    );
    const unreadItems = screen.getAllByRole("button", { name: /^未读/ });
    const openedItems = screen.getAllByRole("button", { name: /^已读/ });
    expect(unreadItems).toHaveLength(3);
    expect(openedItems).toHaveLength(2);
    expect(unreadItems[0]).toHaveAttribute("data-unread", "true");
    expect(unreadItems[0].querySelector(".inbox-unread-mark")).toBeInTheDocument();
    expect(openedItems[0]).toHaveAttribute("data-unread", "false");
    expect(screen.getAllByRole("button")[0]).toHaveTextContent("送达内容 1");
    expect(screen.getByRole("button", { name: /送达内容 1.*Daily Brief.*文章/ })).toBeVisible();
  });

  it("opens an item through the whole accessible paper row", async () => {
    const user = userEvent.setup();
    const onOpen = vi.fn();
    render(
      <InboxList
        items={[item(1, true)]}
        state="ready"
        openingId={null}
        openError={null}
        onRetry={vi.fn()}
        onOpen={onOpen}
      />,
    );
    await user.click(screen.getByRole("button", { name: /^未读/ }));
    expect(onOpen).toHaveBeenCalledWith("delivery-1");
  });
});
