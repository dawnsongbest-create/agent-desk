import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { StickyCardsPort } from "../../application/ports/sticky";
import { defaultPreferences } from "../../domain/preferences";
import type { CreateStickyCardInput, StickyCard } from "../../domain/sticky";
import { playPageTurnSound } from "./pageTurnSound";
import { StickyHome } from "./StickyHome";

vi.mock("./pageTurnSound", () => ({
  playPageTurnSound: vi.fn(),
}));

function makeCard(
  id: string,
  kind: "note" | "task",
  text: string,
  position: number,
  overrides: Partial<StickyCard> = {},
): StickyCard {
  return {
    id,
    kind,
    text,
    completed: false,
    dueDate: null,
    position,
    createdAt: "2026-08-10T00:00:00Z",
    updatedAt: "2026-08-10T00:00:00Z",
    ...overrides,
  };
}

class MemoryStickyPort implements StickyCardsPort {
  cards: StickyCard[];

  constructor(cards: StickyCard[] = []) {
    this.cards = cards.map((card) => ({ ...card }));
  }

  list = vi.fn(async () => this.cards.map((card) => ({ ...card })));

  create = vi.fn(async (input: CreateStickyCardInput) => {
    const created = makeCard(
      `created-${this.cards.length + 1}`,
      input.kind,
      input.text,
      this.cards.length,
      { dueDate: input.dueDate },
    );
    this.cards.push(created);
    return { ...created };
  });

  updateText = vi.fn(async (id: string, text: string) => {
    const card = this.required(id);
    card.text = text;
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

  reorder = vi.fn(async (orderedIds: string[]) => {
    const lookup = new Map(this.cards.map((card) => [card.id, card]));
    this.cards = orderedIds.map((id, position) => ({
      ...this.requiredFrom(lookup, id),
      position,
    }));
    return this.cards.map((card) => ({ ...card }));
  });

  private required(id: string) {
    const card = this.cards.find((item) => item.id === id);
    if (!card) throw new Error("missing card");
    return card;
  }

  private requiredFrom(lookup: Map<string, StickyCard>, id: string) {
    const card = lookup.get(id);
    if (!card) throw new Error("missing card");
    return card;
  }
}

function renderHome(port: StickyCardsPort) {
  return render(
    <StickyHome
      port={port}
      preferences={defaultPreferences}
      preferenceSaveState="idle"
      now={new Date("2026-08-10T12:00:00")}
      onThemeChange={vi.fn()}
      onAlwaysOnTopChange={vi.fn()}
    />,
  );
}

async function expandSticky(user: ReturnType<typeof userEvent.setup>) {
  await user.click(await screen.findByRole("button", { name: "展开便利贴" }));
  return screen.getByRole("region", { name: "展开的便利贴" });
}

describe("StickyHome M1-B2", () => {
  it("starts as a quiet board with a compact sticky preview", async () => {
    renderHome(new MemoryStickyPort());

    expect(await screen.findByRole("button", { name: "展开便利贴" })).toHaveTextContent(
      "纸上还很安静",
    );
    expect(screen.getByRole("heading")).toBeInTheDocument();
    expect(screen.queryByText(/欢迎|AI/)).not.toBeInTheDocument();
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
  });

  it("expands in place and keeps the Note capture after existing Note content", async () => {
    const user = userEvent.setup();
    renderHome(new MemoryStickyPort([makeCard("note-1", "note", "先读完这一章", 0)]));

    const expanded = await expandSticky(user);
    expect(expanded).toBeInTheDocument();
    const note = screen.getByRole("button", { name: "先读完这一章" });
    const capture = screen.getByRole("button", { name: /继续写一条记录/ });
    expect(note.compareDocumentPosition(capture) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it("creates a multiline Note from the content-following Note capture", async () => {
    const user = userEvent.setup();
    const port = new MemoryStickyPort();
    renderHome(port);
    await expandSticky(user);

    await user.click(screen.getByRole("button", { name: /继续写一条记录/ }));
    const editor = screen.getByRole("textbox", { name: "新记录" });
    await user.type(editor, "第一行{enter}第二行");
    await user.click(screen.getByRole("button", { name: "写下" }));

    const matchingButtons = await screen.findAllByRole("button", { name: /第一行\s+第二行/ });
    expect(matchingButtons.some((button) => button.classList.contains("card-text-button"))).toBe(
      true,
    );
    expect(port.create).toHaveBeenCalledWith({
      kind: "note",
      text: "第一行\n第二行",
      dueDate: null,
    });
  });

  it("switches both paper faces with directional sound and an explicit Todo capture", async () => {
    const user = userEvent.setup();
    const port = new MemoryStickyPort();
    renderHome(port);
    await expandSticky(user);

    await user.click(screen.getByRole("tab", { name: /待办/ }));
    expect(screen.getByRole("tabpanel", { name: "待办面" })).toHaveAttribute(
      "data-turn",
      "forward",
    );
    expect(playPageTurnSound).toHaveBeenCalledWith("note-to-todo");
    expect(screen.getByRole("button", { name: /添加一个待办/ })).toBeVisible();

    await user.click(screen.getByRole("button", { name: /添加一个待办/ }));
    await user.type(screen.getByRole("textbox", { name: "新待办" }), "修改公众号稿件");
    await user.click(screen.getByRole("button", { name: "添加" }));
    expect(await screen.findByRole("checkbox", { name: /完成待办：修改公众号稿件/ })).toBeVisible();
    expect(port.create).toHaveBeenCalledWith({
      kind: "task",
      text: "修改公众号稿件",
      dueDate: null,
    });

    await user.click(screen.getByRole("tab", { name: /记录/ }));
    expect(screen.getByRole("tabpanel", { name: "记录面" })).toHaveAttribute(
      "data-turn",
      "backward",
    );
    expect(playPageTurnSound).toHaveBeenLastCalledWith("todo-to-note");
  });

  it("retains quick Todo syntax on the Note face", async () => {
    const user = userEvent.setup();
    const port = new MemoryStickyPort();
    renderHome(port);
    await expandSticky(user);

    await user.click(screen.getByRole("button", { name: /继续写一条记录/ }));
    fireEvent.change(screen.getByRole("textbox", { name: "新记录" }), {
      target: { value: "[ ] 写 PRD" },
    });
    await user.click(screen.getByRole("button", { name: "写下" }));
    expect(port.create).toHaveBeenLastCalledWith({ kind: "task", text: "写 PRD", dueDate: null });
  });

  it("edits multiline Notes and single-line Todos with their existing save semantics", async () => {
    const user = userEvent.setup();
    const port = new MemoryStickyPort([
      makeCard("note-1", "note", "旧记录", 0),
      makeCard("task-1", "task", "旧待办", 1),
    ]);
    renderHome(port);
    await expandSticky(user);

    await user.click(screen.getByRole("button", { name: "旧记录" }));
    const noteEditor = screen.getByRole("textbox", { name: "编辑记录" });
    await user.clear(noteEditor);
    await user.type(noteEditor, "新第一行{enter}新第二行");
    fireEvent.keyDown(noteEditor, { key: "Enter", ctrlKey: true });
    await waitFor(() =>
      expect(port.updateText).toHaveBeenCalledWith("note-1", "新第一行\n新第二行"),
    );

    await user.click(screen.getByRole("tab", { name: /待办/ }));
    await user.click(screen.getByRole("button", { name: "旧待办" }));
    const taskEditor = screen.getByRole("textbox", { name: "编辑待办" });
    await user.clear(taskEditor);
    await user.type(taskEditor, "新待办{enter}");
    await waitFor(() => expect(port.updateText).toHaveBeenCalledWith("task-1", "新待办"));
  });

  it("completes, dates, and deletes a Todo on the Todo face", async () => {
    const user = userEvent.setup();
    const port = new MemoryStickyPort([makeCard("task-1", "task", "整理材料", 0)]);
    renderHome(port);
    await expandSticky(user);
    await user.click(screen.getByRole("tab", { name: /待办/ }));

    await user.click(screen.getByRole("checkbox", { name: /完成待办：整理材料/ }));
    await waitFor(() => expect(port.setTaskCompleted).toHaveBeenCalledWith("task-1", true));

    await user.click(screen.getByRole("button", { name: /更多操作：整理材料/ }));
    const panel = screen.getByLabelText("整理材料 操作");
    await user.type(within(panel).getByLabelText("待办日期：整理材料"), "2026-08-18");
    await waitFor(() => expect(port.setTaskDueDate).toHaveBeenCalledWith("task-1", "2026-08-18"));
    expect(screen.getByText(/18/)).toBeInTheDocument();

    await user.click(within(panel).getByRole("button", { name: "删除" }));
    await waitFor(() => expect(port.delete).toHaveBeenCalledWith("task-1"));
    expect(screen.queryByText("整理材料")).not.toBeInTheDocument();
  });

  it("keeps theme and always-on-top controls wired in the redesigned popover", async () => {
    const user = userEvent.setup();
    const onThemeChange = vi.fn();
    const onAlwaysOnTopChange = vi.fn();
    render(
      <StickyHome
        port={new MemoryStickyPort()}
        preferences={defaultPreferences}
        preferenceSaveState="idle"
        onThemeChange={onThemeChange}
        onAlwaysOnTopChange={onAlwaysOnTopChange}
      />,
    );

    await user.click(await screen.findByRole("button", { name: "外观与窗口设置" }));
    await user.click(screen.getByRole("button", { name: "深色" }));
    expect(onThemeChange).toHaveBeenCalledWith("dark");
    expect(screen.queryByLabelText("外观与窗口设置面板")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "外观与窗口设置" }));
    await user.click(screen.getByRole("button", { name: "将窗口置顶" }));
    expect(onAlwaysOnTopChange).toHaveBeenCalledWith(true);
  });

  it("keeps Todo drag handles permanently rendered and sortable", async () => {
    const user = userEvent.setup();
    renderHome(
      new MemoryStickyPort([
        makeCard("note-1", "note", "阅读摘录", 0),
        makeCard("task-1", "task", "第一件", 1),
        makeCard("task-2", "task", "第二件", 2),
      ]),
    );
    await expandSticky(user);
    await user.click(screen.getByRole("tab", { name: /待办/ }));

    const handles = screen.getAllByRole("button", { name: /拖动排序/ });
    expect(handles).toHaveLength(2);
    expect(handles[0]).toHaveClass("drag-handle");
    expect(handles[0]).toHaveAttribute("aria-roledescription", "sortable");
  });

  it("rolls back optimistic edits and refetches after a mutation failure", async () => {
    const user = userEvent.setup();
    const port = new MemoryStickyPort([makeCard("note-1", "note", "已保存内容", 0)]);
    port.updateText.mockRejectedValueOnce(new Error("disk unavailable"));
    renderHome(port);
    await expandSticky(user);

    await user.click(screen.getByRole("button", { name: "已保存内容" }));
    const editor = screen.getByRole("textbox", { name: "编辑记录" });
    await user.clear(editor);
    await user.type(editor, "不会被保存");
    fireEvent.keyDown(editor, { key: "Enter", ctrlKey: true });

    expect(await screen.findByRole("alert")).toHaveTextContent("已恢复到上次确认的数据");
    expect(screen.getByRole("button", { name: "已保存内容" })).toBeInTheDocument();
    expect(screen.queryByText("不会被保存")).not.toBeInTheDocument();
    expect(port.list).toHaveBeenCalledTimes(2);
  });
});
