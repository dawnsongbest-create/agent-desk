import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { StickyCardsPort } from "../../application/ports/sticky";
import { defaultPreferences } from "../../domain/preferences";
import type { CreateStickyCardInput, StickyCard } from "../../domain/sticky";
import { StickyHome } from "./StickyHome";

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

describe("StickyHome", () => {
  it("keeps the first-run empty state quiet and capture-ready", async () => {
    renderHome(new MemoryStickyPort());

    expect(await screen.findByRole("button", { name: /记点什么/ })).toBeEnabled();
    expect(screen.getByRole("heading", { name: "今天" })).toBeInTheDocument();
    expect(screen.queryByText(/欢迎|AI/)).not.toBeInTheDocument();
  });

  it("creates a multiline Note from unified capture", async () => {
    const user = userEvent.setup();
    const port = new MemoryStickyPort();
    renderHome(port);

    await user.click(await screen.findByRole("button", { name: /记点什么/ }));
    const editor = screen.getByRole("textbox", { name: "新随手记" });
    await user.type(editor, "第一行{enter}第二行");
    await user.click(screen.getByRole("button", { name: "记下" }));

    const matchingButtons = await screen.findAllByRole("button", {
      name: /第一行\s+第二行/,
    });
    expect(matchingButtons.some((button) => button.classList.contains("card-text-button"))).toBe(
      true,
    );
    expect(port.create).toHaveBeenCalledWith({
      kind: "note",
      text: "第一行\n第二行",
      dueDate: null,
    });
  });

  it("creates a Task through both GUI and quick syntax", async () => {
    const user = userEvent.setup();
    const port = new MemoryStickyPort();
    renderHome(port);

    await user.click(await screen.findByRole("button", { name: /记点什么/ }));
    await user.click(screen.getByRole("button", { name: "任务" }));
    await user.type(screen.getByRole("textbox", { name: "新任务" }), "修改公众号稿件");
    await user.click(screen.getByRole("button", { name: "记下" }));
    expect(
      await screen.findByRole("checkbox", { name: /完成任务：修改公众号稿件/ }),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /记点什么/ }));
    fireEvent.change(screen.getByRole("textbox", { name: "新随手记" }), {
      target: { value: "[ ] 写 PRD" },
    });
    await user.click(screen.getByRole("button", { name: "记下" }));
    expect(await screen.findByRole("checkbox", { name: /完成任务：写 PRD/ })).toBeInTheDocument();
    expect(port.create).toHaveBeenLastCalledWith({
      kind: "task",
      text: "写 PRD",
      dueDate: null,
    });
  });

  it("edits multiline Notes and single-line Tasks inline with distinct keyboard behavior", async () => {
    const user = userEvent.setup();
    const port = new MemoryStickyPort([
      makeCard("note-1", "note", "旧随手记", 0),
      makeCard("task-1", "task", "旧任务", 1),
    ]);
    renderHome(port);

    await user.click(await screen.findByRole("button", { name: "旧随手记" }));
    const noteEditor = screen.getByRole("textbox", { name: "编辑随手记" });
    await user.clear(noteEditor);
    await user.type(noteEditor, "新第一行{enter}新第二行");
    fireEvent.keyDown(noteEditor, { key: "Enter", ctrlKey: true });
    await waitFor(() =>
      expect(port.updateText).toHaveBeenCalledWith("note-1", "新第一行\n新第二行"),
    );

    await user.click(screen.getByRole("button", { name: "旧任务" }));
    const taskEditor = screen.getByRole("textbox", { name: "编辑任务" });
    await user.clear(taskEditor);
    await user.type(taskEditor, "新任务{enter}");
    await waitFor(() => expect(port.updateText).toHaveBeenCalledWith("task-1", "新任务"));
  });

  it("completes, restores, dates, and deletes a Task", async () => {
    const user = userEvent.setup();
    const port = new MemoryStickyPort([makeCard("task-1", "task", "整理材料", 0)]);
    renderHome(port);

    const checkbox = await screen.findByRole("checkbox", { name: /完成任务：整理材料/ });
    await user.click(checkbox);
    await waitFor(() => expect(port.setTaskCompleted).toHaveBeenCalledWith("task-1", true));
    await user.click(screen.getByRole("checkbox", { name: /恢复任务：整理材料/ }));
    await waitFor(() => expect(port.setTaskCompleted).toHaveBeenLastCalledWith("task-1", false));

    await user.click(screen.getByRole("button", { name: /更多操作：整理材料/ }));
    const panel = screen.getByLabelText("整理材料 操作");
    await user.type(within(panel).getByLabelText("任务日期：整理材料"), "2026-08-18");
    await waitFor(() => expect(port.setTaskDueDate).toHaveBeenCalledWith("task-1", "2026-08-18"));
    expect(screen.getByText(/18/)).toBeInTheDocument();

    await user.click(within(panel).getByRole("button", { name: "删除" }));
    await waitFor(() => expect(port.delete).toHaveBeenCalledWith("task-1"));
    expect(screen.queryByText("整理材料")).not.toBeInTheDocument();
  });

  it("keeps the M1-A theme and always-on-top settings wired", async () => {
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

    await user.click(await screen.findByRole("button", { name: "便利贴设置" }));
    await user.click(screen.getByRole("button", { name: "深色" }));
    await user.click(screen.getByRole("button", { name: "置顶窗口" }));

    expect(onThemeChange).toHaveBeenCalledWith("dark");
    expect(onAlwaysOnTopChange).toHaveBeenCalledWith(true);
  });

  it("exposes keyboard reorder and persists a complete unified placement array", async () => {
    const port = new MemoryStickyPort([
      makeCard("note-1", "note", "第一条", 0),
      makeCard("task-1", "task", "第二条", 1),
    ]);
    renderHome(port);

    const handle = await screen.findByRole("button", { name: "重新排序：第一条" });
    expect(handle).toHaveAttribute("aria-roledescription", "sortable");

    await port.reorder(["task-1", "note-1"]);
    expect(port.reorder).toHaveBeenCalledWith(["task-1", "note-1"]);
    expect((await port.list()).map((card) => card.id)).toEqual(["task-1", "note-1"]);
  });

  it("rolls back optimistic edits and refetches after a mutation failure", async () => {
    const user = userEvent.setup();
    const port = new MemoryStickyPort([makeCard("note-1", "note", "已保存内容", 0)]);
    port.updateText.mockRejectedValueOnce(new Error("disk unavailable"));
    renderHome(port);

    await user.click(await screen.findByRole("button", { name: "已保存内容" }));
    const editor = screen.getByRole("textbox", { name: "编辑随手记" });
    await user.clear(editor);
    await user.type(editor, "不会被保存");
    fireEvent.keyDown(editor, { key: "Enter", ctrlKey: true });

    expect(await screen.findByRole("alert")).toHaveTextContent("已恢复到上次确认的数据");
    expect(screen.getByRole("button", { name: "已保存内容" })).toBeInTheDocument();
    expect(screen.queryByText("不会被保存")).not.toBeInTheDocument();
    expect(port.list).toHaveBeenCalledTimes(2);
  });
});
