import { createEvent, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { StickyCardsPort } from "../../application/ports/sticky";
import type { ReaderDocumentsPort } from "../../application/ports/reader";
import type { DeliveriesPort } from "../../application/ports/delivery";
import type { ReadingPlansPort } from "../../application/ports/reading";
import type { AgentProposalsPort } from "../../application/ports/proposal";
import type { AgentConnectionPort } from "../../application/ports/agentConnection";
import { defaultPreferences, type Preferences } from "../../domain/preferences";
import type { ReaderDocument } from "../../domain/reader";
import type { CreateStickyCardInput, StickyCard, StickyProfile } from "../../domain/sticky";
import type { InboxDelivery } from "../../domain/delivery";
import type { AgentProposal } from "../../domain/proposal";
import { playPageTurnSound } from "./pageTurnSound";
import { StickyHome } from "./StickyHome";

vi.mock("./pageTurnSound", () => ({ playPageTurnSound: vi.fn() }));

const readerDocument: ReaderDocument = {
  id: "reader-test",
  documentType: "article",
  title: "测试阅读文档",
  subtitle: null,
  contentMarkdown: "Reader 正文",
  sourceType: "builtin",
  sourceLabel: "测试",
  createdAt: "2026-08-12T00:00:00Z",
  updatedAt: "2026-08-12T00:00:00Z",
};

const readerPort: ReaderDocumentsPort = {
  openCurrent: vi.fn(async () => readerDocument),
  get: vi.fn(async () => readerDocument),
  list: vi.fn(async () => [readerDocument]),
  create: vi.fn(async () => readerDocument),
  captureSelection: vi.fn(),
  copyText: vi.fn(),
};

const deliveryPort: DeliveriesPort = {
  ingest: vi.fn(),
  listInbox: vi.fn(async () => []),
  getUnreadCount: vi.fn(async () => 0),
  open: vi.fn(),
};

const readingPort: ReadingPlansPort = {
  createPlan: vi.fn(),
  listPlans: vi.fn(async () => []),
  setPlanStatus: vi.fn(),
  generateToday: vi.fn(),
  createSession: vi.fn(),
};

const proposalPort: AgentProposalsPort = {
  listForDocument: vi.fn(async () => []),
  accept: vi.fn(),
  reject: vi.fn(),
};

const agentConnectionPort: AgentConnectionPort = {
  getStatus: vi.fn(async () => ({
    version: "v1" as const,
    enabled: false,
    running: false,
    bindAddress: "127.0.0.1" as const,
    port: 47321,
    endpoint: "http://127.0.0.1:47321/api/v1",
    connection: null,
    lastError: null,
  })),
  start: vi.fn(),
  stop: vi.fn(),
  generateToken: vi.fn(),
  copyToken: vi.fn(),
};

function makeCard(
  id: string,
  kind: "note" | "task",
  text: string,
  position: number,
  completed = false,
): StickyCard {
  return {
    id,
    kind,
    text,
    completed,
    dueDate: null,
    position,
    createdAt: "2026-08-10T00:00:00Z",
    updatedAt: "2026-08-12T00:00:00Z",
  };
}

class MemoryStickyPort implements StickyCardsPort {
  cards: StickyCard[];
  profile: StickyProfile = { quoteText: "", updatedAt: "2026-08-12T00:00:00Z" };
  constructor(cards: StickyCard[] = []) {
    this.cards = cards.map((card) => ({ ...card }));
  }
  list = vi.fn(async () => this.cards.map((card) => ({ ...card })));
  getProfile = vi.fn(async () => ({ ...this.profile }));
  create = vi.fn(async (input: CreateStickyCardInput) => {
    const card = makeCard(
      `created-${this.cards.length + 1}`,
      input.kind,
      input.text,
      this.cards.length,
    );
    card.dueDate = input.dueDate;
    this.cards.push(card);
    return { ...card };
  });
  updateText = vi.fn(async (id: string, text: string) => {
    const card = this.required(id);
    card.text = text;
    card.updatedAt = "2026-08-13T00:00:00Z";
    return { ...card };
  });
  setTaskCompleted = vi.fn(async (id: string, completed: boolean) => {
    const card = this.required(id);
    card.completed = completed;
    return { ...card };
  });
  setTaskDueDate = vi.fn(async (id: string, dueDate: string | null) => {
    const card = this.required(id);
    card.dueDate = dueDate;
    return { ...card };
  });
  delete = vi.fn(async (id: string) => {
    this.cards = this.cards.filter((card) => card.id !== id);
  });
  reorder = vi.fn(async (ids: string[]) => {
    const lookup = new Map(this.cards.map((card) => [card.id, card]));
    this.cards = ids.map((id, position) => ({ ...lookup.get(id)!, position }));
    return this.cards;
  });
  updateQuote = vi.fn(async (quoteText: string) => {
    this.profile = { quoteText, updatedAt: "2026-08-13T00:00:00Z" };
    return { ...this.profile };
  });
  exportRecord = vi.fn(async () => true);
  private required(id: string) {
    const card = this.cards.find((item) => item.id === id);
    if (!card) throw new Error("missing card");
    return card;
  }
}

function renderHome(
  port: StickyCardsPort,
  positionChange = vi.fn(),
  preferences: Preferences = defaultPreferences,
  modeChange = vi.fn(),
  readerFontSizeChange = vi.fn(),
  readerLineSpacingChange = vi.fn(),
  readerDocuments: ReaderDocumentsPort = readerPort,
  deliveries: DeliveriesPort = deliveryPort,
  proposals: AgentProposalsPort = proposalPort,
) {
  return render(
    <StickyHome
      port={port}
      readerPort={readerDocuments}
      deliveryPort={deliveries}
      readingPort={readingPort}
      proposalPort={proposals}
      agentConnectionPort={agentConnectionPort}
      preferences={preferences}
      preferenceSaveState="idle"
      now={new Date("2026-08-12T12:00:00")}
      onThemeChange={vi.fn()}
      onAlwaysOnTopChange={vi.fn()}
      onWindowPresetChange={vi.fn()}
      onStickyPositionChange={positionChange}
      onStickyModeChange={modeChange}
      onReaderFontSizeChange={readerFontSizeChange}
      onReaderLineSpacingChange={readerLineSpacingChange}
      onReaderContentVisibilityChange={vi.fn()}
      onCurrentReaderDocumentChange={vi.fn()}
    />,
  );
}

function todoProposal(id: string, content: string): AgentProposal {
  return {
    id,
    type: "todo",
    title: content,
    description: "Agent 从会议总结中提取的下一步行动。",
    payloadJson: JSON.stringify({ content }),
    sourceDeliveryId: "delivery-test",
    status: "pending",
    createdAt: "2026-08-22T08:00:00Z",
    resolvedAt: null,
  };
}

class MemoryDeliveryPort implements DeliveriesPort {
  items: InboxDelivery[];
  constructor(items: InboxDelivery[]) {
    this.items = items;
  }
  ingest = vi.fn();
  listInbox = vi.fn(async () =>
    this.items.map((item) => ({
      ...item,
      delivery: { ...item.delivery },
      document: { ...item.document },
    })),
  );
  getUnreadCount = vi.fn(
    async () => this.items.filter((item) => item.delivery.openedAt === null).length,
  );
  open = vi.fn(async (id: string) => {
    const item = this.items.find((candidate) => candidate.delivery.id === id);
    if (!item) throw new Error("missing delivery");
    item.delivery.openedAt ??= "2026-08-15T10:00:00.000Z";
    return { ...item, delivery: { ...item.delivery }, document: { ...item.document } };
  });
}

function inboxDelivery(id: string, title: string): InboxDelivery {
  return {
    delivery: {
      id,
      documentId: `reader-${id}`,
      idempotencyKey: `key-${id}`,
      deliveredAt: "2026-08-15T08:00:00.000Z",
      openedAt: null,
    },
    document: {
      id: `reader-${id}`,
      documentType: "brief",
      title,
      subtitle: "由 Delivery 送达",
      contentMarkdown: "# 新文档\n\n这是从收件箱打开的正文。",
      sourceType: "agent",
      sourceLabel: "Daily Brief",
      createdAt: "2026-08-15T08:00:00.000Z",
      updatedAt: "2026-08-15T08:00:00.000Z",
    },
  };
}

function dispatchPointer(
  element: Element,
  type: "pointerDown" | "pointerMove" | "pointerUp",
  values: { pointerId: number; clientX: number; clientY: number },
) {
  const event = createEvent[type](element);
  for (const [key, value] of Object.entries(values))
    Object.defineProperty(event, key, { configurable: true, value });
  fireEvent(element, event);
}

async function expand(user: ReturnType<typeof userEvent.setup>) {
  await user.click(await screen.findByRole("button", { name: "展开或拖动便利贴" }));
  return screen.getByRole("region", { name: "展开的便利贴" });
}

describe("StickyHome M1-B4", () => {
  it("adds three reviewed Agent suggestions to Sticky Todo only after acceptance", async () => {
    const user = userEvent.setup();
    const sticky = new MemoryStickyPort();
    const suggested = [
      todoProposal("proposal-1", "整理会议纪要"),
      todoProposal("proposal-2", "确认下周负责人"),
      todoProposal("proposal-3", "发送项目时间表"),
    ];
    const proposals: AgentProposalsPort = {
      listForDocument: vi.fn(async () => suggested),
      accept: vi.fn(async (id: string) => {
        const proposal = suggested.find((item) => item.id === id)!;
        proposal.status = "accepted";
        return {
          proposal: { ...proposal },
          card: makeCard(`card-${id}`, "task", proposal.title, sticky.cards.length),
          readingSession: null,
        };
      }),
      reject: vi.fn(),
    };
    renderHome(
      sticky,
      vi.fn(),
      defaultPreferences,
      vi.fn(),
      vi.fn(),
      vi.fn(),
      readerPort,
      deliveryPort,
      proposals,
    );

    const proposalRegion = await screen.findByRole("complementary", { name: "Agent 建议" });
    expect(within(proposalRegion).getAllByRole("button", { name: "加入待办" })).toHaveLength(3);
    for (const button of within(proposalRegion).getAllByRole("button", { name: "加入待办" })) {
      await user.click(button);
    }
    await expand(user);
    await user.click(screen.getByRole("tab", { name: /待办/ }));
    expect(screen.getByText("整理会议纪要")).toBeVisible();
    expect(screen.getByText("确认下周负责人")).toBeVisible();
    expect(screen.getByText("发送项目时间表")).toBeVisible();
  });

  it("navigates Reader to Inbox and opens a Delivery into visible Reader content", async () => {
    const user = userEvent.setup();
    const deliveries = new MemoryDeliveryPort([inboxDelivery("a", "Agent 今日简报")]);
    renderHome(
      new MemoryStickyPort(),
      vi.fn(),
      defaultPreferences,
      vi.fn(),
      vi.fn(),
      vi.fn(),
      readerPort,
      deliveries,
    );
    const inboxButton = await screen.findByRole("button", {
      name: "打开收件箱，1 件未打开",
    });
    const readerViewport = screen.getByRole("region", { name: "Reader scroll viewport" });
    readerViewport.scrollTop = 640;
    fireEvent.scroll(readerViewport);
    expect(inboxButton).toHaveTextContent("收件 1");
    await user.click(inboxButton);
    expect(screen.getByText("收件箱")).toBeVisible();
    expect(screen.getByRole("region", { name: "Inbox scroll viewport" })).toBeVisible();
    await user.click(screen.getByRole("button", { name: /未读，Agent 今日简报/ }));
    expect(deliveries.open).toHaveBeenCalledWith("a");
    expect(await screen.findByRole("article", { name: "Agent 今日简报" })).toBeVisible();
    await waitFor(() => expect(readerViewport.scrollTop).toBe(0));
    expect(screen.getByText("这是从收件箱打开的正文。")).toBeVisible();
    expect(screen.getByRole("button", { name: "打开收件箱" })).toHaveTextContent("收件");
  });

  it("keeps Inbox unread and active when opening a Delivery fails", async () => {
    const user = userEvent.setup();
    const deliveries = new MemoryDeliveryPort([inboxDelivery("broken", "无法打开的简报")]);
    deliveries.open.mockRejectedValueOnce(new Error("fixture failure"));
    renderHome(
      new MemoryStickyPort(),
      vi.fn(),
      defaultPreferences,
      vi.fn(),
      vi.fn(),
      vi.fn(),
      readerPort,
      deliveries,
    );
    await user.click(await screen.findByRole("button", { name: "打开收件箱，1 件未打开" }));
    await user.click(screen.getByRole("button", { name: /未读，无法打开的简报/ }));
    expect(screen.getByRole("alert")).toHaveTextContent("无法打开这份内容，请重试。");
    expect(screen.getByRole("region", { name: "Inbox scroll viewport" })).toBeVisible();
    expect(screen.getByRole("button", { name: "返回阅读" })).toHaveTextContent("阅读");
  });

  it("restores the same Reader scroll after an Inbox round trip", async () => {
    const user = userEvent.setup();
    renderHome(new MemoryStickyPort());
    const viewport = await screen.findByRole("region", { name: "Reader scroll viewport" });
    viewport.scrollTop = 518;
    fireEvent.scroll(viewport);
    await user.click(screen.getByRole("button", { name: "打开收件箱" }));
    expect(screen.getByRole("region", { name: "Inbox scroll viewport" })).toBe(viewport);
    await user.click(screen.getByRole("button", { name: "返回阅读" }));
    await waitFor(() => expect(viewport.scrollTop).toBe(518));
  });

  it("preserves Blank Reader when visiting Inbox and returning with 阅读", async () => {
    const user = userEvent.setup();
    renderHome(
      new MemoryStickyPort(),
      vi.fn(),
      { ...defaultPreferences, readerContentVisible: false },
      vi.fn(),
      vi.fn(),
      vi.fn(),
      readerPort,
      new MemoryDeliveryPort([]),
    );
    await user.click(await screen.findByRole("button", { name: "打开收件箱" }));
    expect(screen.getByText("暂时没有新内容。")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "返回阅读" }));
    expect(screen.queryByRole("article")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "显示正文" })).toBeVisible();
  });
  it("adds a Reader selection capture to the existing 记录 list without opening Sticky", async () => {
    const user = userEvent.setup();
    const port = new MemoryStickyPort();
    const capturedRecord = makeCard("captured", "note", "Reader 正文", 0);
    const captureSelection = vi.fn(async () => ({
      record: capturedRecord,
      sourceRef: {
        recordId: capturedRecord.id,
        documentId: readerDocument.id,
        sourceType: "reader_selection" as const,
        selectedText: capturedRecord.text,
        documentTitleSnapshot: readerDocument.title,
        capturedAt: "2026-08-15T00:00:00Z",
      },
    }));
    const selectionPort = { ...readerPort, captureSelection };
    renderHome(port, vi.fn(), defaultPreferences, vi.fn(), vi.fn(), vi.fn(), selectionPort);
    const article = await screen.findByRole("article", { name: readerDocument.title });
    const textNode = screen.getByText("Reader 正文").firstChild!;
    const selection = vi.spyOn(window, "getSelection").mockReturnValue({
      rangeCount: 1,
      isCollapsed: false,
      anchorNode: textNode,
      focusNode: textNode,
      toString: () => "Reader 正文",
      getRangeAt: () => ({
        getBoundingClientRect: () =>
          ({ left: 40, right: 130, top: 180, bottom: 200, width: 90, height: 20 }) as DOMRect,
      }),
    } as unknown as Selection);
    fireEvent.mouseUp(article);
    await user.click(screen.getByRole("button", { name: "保存到记录" }));
    expect(captureSelection).toHaveBeenCalledWith({
      documentId: readerDocument.id,
      selectedText: "Reader 正文",
    });
    expect(screen.queryByRole("region", { name: "展开的便利贴" })).not.toBeInTheDocument();
    await expand(user);
    expect(screen.getByRole("button", { name: /Reader 正文/ })).toBeVisible();
    selection.mockRestore();
  });

  it("renders a compact Todo + Quote overlay with only open Todo priority", async () => {
    const port = new MemoryStickyPort([
      makeCard("done", "task", "已完成，不应抢占", 0, true),
      makeCard("one", "task", "第一件", 1),
      makeCard("two", "task", "第二件", 2),
      makeCard("three", "task", "第三件", 3),
      makeCard("four", "task", "第四件", 4),
      makeCard("record", "note", "不应出现在 Compact 的长 Record", 5),
    ]);
    port.profile.quoteText = "多花点时间玩。";
    renderHome(port);
    const compact = await screen.findByRole("button", { name: "展开或拖动便利贴" });
    expect(compact).toHaveTextContent("第一件");
    const viewport = within(compact).getByLabelText("Compact Todo 列表");
    expect(viewport.children).toHaveLength(4);
    expect(viewport.children[0]).toHaveTextContent("第一件");
    expect(viewport.children[3]).toHaveTextContent("第四件");
    expect(within(viewport).getAllByText("", { selector: ".preview-check" })).toHaveLength(4);
    expect(compact).toHaveTextContent("多花点时间玩。");
    expect(compact).not.toHaveTextContent("不应出现在 Compact");
    expect(compact).not.toHaveTextContent("已完成，不应抢占");
    expect(viewport).toHaveClass("compact-todo-viewport");
  }, 15_000);

  it("opens a Record list, creates 5000+ Chinese characters, and reopens the editor", async () => {
    const user = userEvent.setup();
    const port = new MemoryStickyPort([
      makeCard("existing", "note", "Agent Desk 产品想法\n这是摘要", 0),
    ]);
    renderHome(port);
    await expand(user);
    expect(screen.getByText("Agent Desk 产品想法")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /新建记录/ }));
    const body = `长记录第一行\n${"中文内容".repeat(1300)}`;
    fireEvent.change(screen.getByRole("textbox", { name: "新建长记录" }), {
      target: { value: body },
    });
    await user.click(screen.getByRole("button", { name: "保存记录" }));
    await waitFor(() =>
      expect(port.create).toHaveBeenCalledWith({ kind: "note", text: body, dueDate: null }),
    );
    await user.click(screen.getByRole("button", { name: /长记录第一行/ }));
    expect(screen.getByRole("textbox", { name: "记录正文" })).toHaveValue(body);
  });

  it("edits and saves a long Record without leaving the paper", async () => {
    const user = userEvent.setup();
    const port = new MemoryStickyPort([makeCard("record", "note", "原始标题\n原始正文", 0)]);
    renderHome(port);
    await expand(user);
    await user.click(screen.getByRole("button", { name: /原始标题/ }));
    const editor = screen.getByRole("textbox", { name: "记录正文" });
    fireEvent.change(editor, { target: { value: "修改后标题\n修改后的正文" } });
    await user.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() =>
      expect(port.updateText).toHaveBeenCalledWith("record", "修改后标题\n修改后的正文"),
    );
    expect(screen.getByText("已保存")).toBeInTheDocument();
  });

  it("releases Record navigation after an ordinary Save completes", async () => {
    const user = userEvent.setup();
    const port = new MemoryStickyPort([
      makeCard("record", "note", "需要保存的 Record", 0),
      makeCard("todo", "task", "保存后可以打开", 1),
    ]);
    renderHome(port);
    await expand(user);
    await user.click(screen.getByRole("button", { name: /需要保存的 Record/ }));
    fireEvent.change(screen.getByRole("textbox", { name: "记录正文" }), {
      target: { value: "普通保存后的内容" },
    });
    expect(screen.getByRole("tab", { name: /待办/ })).toBeDisabled();
    expect(screen.getByRole("button", { name: /新记录/ })).toBeDisabled();
    expect(screen.getByRole("button", { name: "收起便利贴" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(screen.getByText("已保存")).toBeInTheDocument());

    const todoTab = screen.getByRole("tab", { name: /待办/ });
    expect(todoTab).toBeEnabled();
    const newRecord = screen.getByRole("button", { name: /新记录/ });
    expect(newRecord).toBeEnabled();
    expect(screen.getByRole("button", { name: "收起便利贴" })).toBeEnabled();
    await user.click(newRecord);
    expect(await screen.findByRole("textbox", { name: "新建长记录" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "取消" }));
    await user.click(todoTab);
    expect(screen.getByText("保存后可以打开")).toBeInTheDocument();
  });

  it("releases and persists a 6000+ character Record after ordinary Save", async () => {
    const user = userEvent.setup();
    const body = `长 Record 普通保存\n${"六千字中文保存状态验证".repeat(650)}`;
    const port = new MemoryStickyPort([makeCard("record", "note", "长 Record 原文", 0)]);
    renderHome(port);
    await expand(user);
    await user.click(screen.getByRole("button", { name: /长 Record 原文/ }));
    fireEvent.change(screen.getByRole("textbox", { name: "记录正文" }), {
      target: { value: body },
    });
    await user.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(port.updateText).toHaveBeenCalledWith("record", body));
    await waitFor(() => expect(screen.getByText("已保存")).toBeInTheDocument());
    const collapse = screen.getByRole("button", { name: "收起便利贴" });
    expect(collapse).toBeEnabled();
    await user.click(collapse);
    await user.click(screen.getByRole("button", { name: "展开或拖动便利贴" }));
    await user.click(screen.getByRole("button", { name: /长 Record 普通保存/ }));
    expect(screen.getByRole("textbox", { name: "记录正文" })).toHaveValue(body);
  });

  it("pastes 6000+ Chinese characters, saves, and collapses without controlled rerenders", async () => {
    const user = userEvent.setup();
    const original = "原始长记录";
    const pasted = `稳定性记录\n${"大段中文内容".repeat(1100)}`;
    const port = new MemoryStickyPort([makeCard("record", "note", original, 0)]);
    renderHome(port);
    await expand(user);
    await user.click(screen.getByRole("button", { name: /原始长记录/ }));
    const editor = screen.getByRole("textbox", { name: "记录正文" });
    fireEvent.change(editor, { target: { value: pasted } });
    expect(screen.getByText(pasted.length.toLocaleString())).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "保存并收起" }));
    await waitFor(() => expect(port.updateText).toHaveBeenCalledWith("record", pasted));
    expect(screen.getByRole("button", { name: "展开或拖动便利贴" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "展开或拖动便利贴" }));
    await user.click(screen.getByRole("button", { name: /稳定性记录/ }));
    expect(screen.getByRole("textbox", { name: "记录正文" })).toHaveValue(pasted);
  });

  it("stores the full Sticky Quote beyond the compact visual line limit", async () => {
    const user = userEvent.setup();
    const port = new MemoryStickyPort();
    const quote =
      "愿你在很长很长的日子里，仍然保留好奇、耐心和一点点不合时宜的浪漫，并记得给真正重要的事留出时间。";
    renderHome(port);
    await expand(user);
    await user.click(screen.getByRole("button", { name: "写下你的便签一句" }));
    fireEvent.change(screen.getByRole("textbox", { name: "编辑便签一句" }), {
      target: { value: quote },
    });
    await user.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(port.updateQuote).toHaveBeenCalledWith(quote));
    expect(port.profile.quoteText).toBe(quote);
    await user.click(screen.getByRole("button", { name: "收起便利贴" }));
    expect(await screen.findByText(quote)).toBeInTheDocument();
  });

  it("exports the selected saved Record through the native boundary", async () => {
    const user = userEvent.setup();
    const port = new MemoryStickyPort([makeCard("record", "note", "可导出的 Record", 0)]);
    renderHome(port);
    await expand(user);
    await user.click(screen.getByRole("button", { name: /可导出的 Record/ }));
    await user.click(screen.getByRole("button", { name: "导出 .md" }));
    expect(port.exportRecord).toHaveBeenCalledWith("record");
  });

  it("keeps the existing directional page turn and sound", async () => {
    const user = userEvent.setup();
    renderHome(new MemoryStickyPort());
    await expand(user);
    await user.click(screen.getByRole("tab", { name: /待办/ }));
    expect(playPageTurnSound).toHaveBeenLastCalledWith("note-to-todo");
    await user.click(screen.getByRole("tab", { name: /记录/ }));
    expect(playPageTurnSound).toHaveBeenLastCalledWith("todo-to-note");
  });

  it("distinguishes a sub-threshold click from Compact drag", async () => {
    renderHome(new MemoryStickyPort());
    const compact = await screen.findByRole("button", { name: "展开或拖动便利贴" });
    fireEvent.pointerDown(compact, { pointerId: 1, clientX: 100, clientY: 100 });
    fireEvent.pointerUp(compact, { pointerId: 1, clientX: 103, clientY: 102 });
    expect(screen.getByRole("region", { name: "展开的便利贴" })).toBeInTheDocument();
  });

  it("keeps Compact drag continuous and does not expand after crossing the threshold", async () => {
    const positionChange = vi.fn();
    renderHome(new MemoryStickyPort(), positionChange);
    const compact = await screen.findByRole("button", { name: "展开或拖动便利贴" });
    const board = compact.parentElement!;
    vi.spyOn(compact, "getBoundingClientRect").mockReturnValue({
      x: 80,
      y: 120,
      left: 80,
      top: 120,
      right: 324,
      bottom: 370,
      width: 244,
      height: 250,
      toJSON: () => ({}),
    });
    vi.spyOn(board, "getBoundingClientRect").mockReturnValue({
      x: 0,
      y: 0,
      left: 0,
      top: 0,
      right: 420,
      bottom: 594,
      width: 420,
      height: 594,
      toJSON: () => ({}),
    });
    Object.defineProperties(compact, {
      offsetWidth: { configurable: true, value: 244 },
      offsetHeight: { configurable: true, value: 250 },
    });
    dispatchPointer(compact, "pointerDown", { pointerId: 2, clientX: 100, clientY: 140 });
    dispatchPointer(compact, "pointerMove", { pointerId: 2, clientX: 106, clientY: 148 });
    expect(compact).toHaveStyle({ left: "86px", top: "128px" });
    dispatchPointer(compact, "pointerUp", { pointerId: 2, clientX: 106, clientY: 148 });
    expect(positionChange).toHaveBeenCalledOnce();
    expect(screen.queryByRole("region", { name: "展开的便利贴" })).not.toBeInTheDocument();
  });

  it("switches Compact to Mini Tab and restores Compact without state conflict", async () => {
    const user = userEvent.setup();
    const modeChange = vi.fn();
    const positionChange = vi.fn();
    const port = new MemoryStickyPort([makeCard("todo", "task", "一件事", 0)]);
    const view = renderHome(port, positionChange, defaultPreferences, modeChange);
    await user.click(await screen.findByRole("button", { name: "缩成 Mini Tab" }));
    expect(modeChange).toHaveBeenCalledWith("mini");
    expect(positionChange).not.toHaveBeenCalled();
    view.rerender(
      <StickyHome
        port={port}
        readerPort={readerPort}
        deliveryPort={deliveryPort}
        readingPort={readingPort}
        proposalPort={proposalPort}
        agentConnectionPort={agentConnectionPort}
        preferences={{ ...defaultPreferences, stickyMode: "mini" }}
        preferenceSaveState="idle"
        now={new Date("2026-08-12T12:00:00")}
        onThemeChange={vi.fn()}
        onAlwaysOnTopChange={vi.fn()}
        onWindowPresetChange={vi.fn()}
        onStickyPositionChange={vi.fn()}
        onStickyModeChange={modeChange}
        onReaderFontSizeChange={vi.fn()}
        onReaderLineSpacingChange={vi.fn()}
        onReaderContentVisibilityChange={vi.fn()}
        onCurrentReaderDocumentChange={vi.fn()}
      />,
    );
    const mini = screen.getByRole("button", { name: "恢复 Compact Sticky" });
    expect(mini).toHaveTextContent("TODAY1");
    expect(mini).toHaveAttribute("data-pinned-to-shelf", "true");
    await user.click(mini);
    expect(modeChange).toHaveBeenLastCalledWith("compact");
  });

  it("pins Mini to the Safe Shelf without changing the saved Compact position", async () => {
    const modeChange = vi.fn();
    const positionChange = vi.fn();
    renderHome(
      new MemoryStickyPort(),
      positionChange,
      { ...defaultPreferences, stickyMode: "mini" },
      modeChange,
    );
    const mini = await screen.findByRole("button", {
      name: "恢复 Compact Sticky",
    });
    dispatchPointer(mini, "pointerDown", { pointerId: 3, clientX: 110, clientY: 210 });
    dispatchPointer(mini, "pointerMove", { pointerId: 3, clientX: 140, clientY: 240 });
    dispatchPointer(mini, "pointerUp", { pointerId: 3, clientX: 140, clientY: 240 });
    expect(positionChange).not.toHaveBeenCalled();
    expect(modeChange).not.toHaveBeenCalled();
    expect(mini).toHaveAttribute("data-pinned-to-shelf", "true");
  });

  it("offers all four Window Size Presets with Mini Tab production behavior", async () => {
    const user = userEvent.setup();
    renderHome(new MemoryStickyPort());
    await user.click(screen.getByRole("button", { name: "外观与窗口设置" }));
    for (const label of ["Sticky", "iPhone 5", "Pocket", "Book"])
      expect(screen.getByRole("button", { name: label })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "缩成 Mini Tab" })).toBeInTheDocument();
  });

  it("keeps Reader below the Compact overlay and forwards boundary wheel movement", async () => {
    const port = new MemoryStickyPort([
      makeCard("one", "task", "第一件", 0),
      makeCard("two", "task", "第二件", 1),
      makeCard("three", "task", "第三件", 2),
      makeCard("four", "task", "第四件", 3),
    ]);
    renderHome(port);
    const reader = await screen.findByRole("region", { name: "Reader Canvas" });
    const readerViewport = screen.getByRole("region", { name: "Reader scroll viewport" });
    const compact = screen.getByRole("button", { name: "展开或拖动便利贴" });
    expect(reader).toHaveAttribute("data-reader-layer", "1");
    expect(compact.parentElement).toHaveAttribute("data-reader-layer", "2");

    const todo = within(compact).getByLabelText("Compact Todo 列表");
    Object.defineProperties(todo, {
      clientHeight: { configurable: true, value: 70 },
      scrollHeight: { configurable: true, value: 140 },
      scrollTop: { configurable: true, writable: true, value: 20 },
    });
    readerViewport.scrollTop = 100;
    fireEvent.wheel(todo, { deltaY: 18 });
    expect(todo.scrollTop).toBe(38);
    expect(readerViewport.scrollTop).toBe(100);

    todo.scrollTop = 70;
    fireEvent.wheel(todo, { deltaY: 18 });
    expect(readerViewport.scrollTop).toBe(118);
  });

  it("offers keyboard-accessible Reader font size and line spacing controls", async () => {
    const user = userEvent.setup();
    const fontSizeChange = vi.fn();
    const lineSpacingChange = vi.fn();
    renderHome(
      new MemoryStickyPort(),
      vi.fn(),
      defaultPreferences,
      vi.fn(),
      fontSizeChange,
      lineSpacingChange,
    );
    await user.click(screen.getByRole("button", { name: "外观与窗口设置" }));
    const fontGroup = screen.getByRole("group", { name: "Reader 字号" });
    const spacingGroup = screen.getByRole("group", { name: "Reader 行距" });
    expect(within(fontGroup).getByRole("button", { name: "标准" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    await user.click(within(fontGroup).getByRole("button", { name: "大" }));
    await user.click(within(spacingGroup).getByRole("button", { name: "宽松" }));
    expect(fontSizeChange).toHaveBeenCalledWith("large");
    expect(lineSpacingChange).toHaveBeenCalledWith("relaxed");
  });
});
