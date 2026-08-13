import { createEvent, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { StickyCardsPort } from "../../application/ports/sticky";
import { defaultPreferences, type Preferences } from "../../domain/preferences";
import type { CreateStickyCardInput, StickyCard, StickyProfile } from "../../domain/sticky";
import { playPageTurnSound } from "./pageTurnSound";
import { StickyHome } from "./StickyHome";

vi.mock("./pageTurnSound", () => ({ playPageTurnSound: vi.fn() }));

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
) {
  return render(
    <StickyHome
      port={port}
      preferences={preferences}
      preferenceSaveState="idle"
      now={new Date("2026-08-12T12:00:00")}
      onThemeChange={vi.fn()}
      onAlwaysOnTopChange={vi.fn()}
      onWindowPresetChange={vi.fn()}
      onStickyPositionChange={positionChange}
      onStickyModeChange={modeChange}
    />,
  );
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
    await user.click(screen.getByRole("button", { name: /新建 Record/ }));
    const body = `长记录第一行\n${"中文内容".repeat(1300)}`;
    fireEvent.change(screen.getByRole("textbox", { name: "新建长 Record" }), {
      target: { value: body },
    });
    await user.click(screen.getByRole("button", { name: "保存 Record" }));
    await waitFor(() =>
      expect(port.create).toHaveBeenCalledWith({ kind: "note", text: body, dueDate: null }),
    );
    await user.click(screen.getByRole("button", { name: /长记录第一行/ }));
    expect(screen.getByRole("textbox", { name: "Record 正文" })).toHaveValue(body);
  });

  it("edits and saves a long Record without leaving the paper", async () => {
    const user = userEvent.setup();
    const port = new MemoryStickyPort([makeCard("record", "note", "原始标题\n原始正文", 0)]);
    renderHome(port);
    await expand(user);
    await user.click(screen.getByRole("button", { name: /原始标题/ }));
    const editor = screen.getByRole("textbox", { name: "Record 正文" });
    fireEvent.change(editor, { target: { value: "修改后标题\n修改后的正文" } });
    await user.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() =>
      expect(port.updateText).toHaveBeenCalledWith("record", "修改后标题\n修改后的正文"),
    );
    expect(screen.getByText("已保存")).toBeInTheDocument();
  });

  it("pastes 6000+ Chinese characters, saves, and collapses without controlled rerenders", async () => {
    const user = userEvent.setup();
    const original = "原始长记录";
    const pasted = `稳定性记录\n${"大段中文内容".repeat(1100)}`;
    const port = new MemoryStickyPort([makeCard("record", "note", original, 0)]);
    renderHome(port);
    await expand(user);
    await user.click(screen.getByRole("button", { name: /原始长记录/ }));
    const editor = screen.getByRole("textbox", { name: "Record 正文" });
    fireEvent.change(editor, { target: { value: pasted } });
    expect(screen.getByText(pasted.length.toLocaleString())).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "保存并收起" }));
    await waitFor(() => expect(port.updateText).toHaveBeenCalledWith("record", pasted));
    expect(screen.getByRole("button", { name: "展开或拖动便利贴" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "展开或拖动便利贴" }));
    await user.click(screen.getByRole("button", { name: /稳定性记录/ }));
    expect(screen.getByRole("textbox", { name: "Record 正文" })).toHaveValue(pasted);
  });

  it("updates the single Sticky Quote and shows it after collapse", async () => {
    const user = userEvent.setup();
    const port = new MemoryStickyPort();
    renderHome(port);
    await expand(user);
    await user.click(screen.getByRole("button", { name: "写下你的便签一句" }));
    fireEvent.change(screen.getByRole("textbox", { name: "编辑便签一句" }), {
      target: { value: "多花点时间玩。" },
    });
    await user.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(port.updateQuote).toHaveBeenCalledWith("多花点时间玩。"));
    await user.click(screen.getByRole("button", { name: "收起便利贴" }));
    expect(await screen.findByText("多花点时间玩。")).toBeInTheDocument();
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
    await user.click(screen.getByRole("tab", { name: /Record/ }));
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
    const port = new MemoryStickyPort([makeCard("todo", "task", "一件事", 0)]);
    const view = renderHome(port, vi.fn(), defaultPreferences, modeChange);
    await user.click(await screen.findByRole("button", { name: "缩成 Mini Tab" }));
    expect(modeChange).toHaveBeenCalledWith("mini");
    view.rerender(
      <StickyHome
        port={port}
        preferences={{ ...defaultPreferences, stickyMode: "mini" }}
        preferenceSaveState="idle"
        now={new Date("2026-08-12T12:00:00")}
        onThemeChange={vi.fn()}
        onAlwaysOnTopChange={vi.fn()}
        onWindowPresetChange={vi.fn()}
        onStickyPositionChange={vi.fn()}
        onStickyModeChange={modeChange}
      />,
    );
    const mini = screen.getByRole("button", { name: "恢复 Compact Sticky 或拖动 Mini Tab" });
    expect(mini).toHaveTextContent("TODAY1");
    await user.click(mini);
    expect(modeChange).toHaveBeenLastCalledWith("compact");
  });

  it("drags Mini Tab without restoring Compact", async () => {
    const modeChange = vi.fn();
    const positionChange = vi.fn();
    renderHome(
      new MemoryStickyPort(),
      positionChange,
      { ...defaultPreferences, stickyMode: "mini" },
      modeChange,
    );
    const mini = await screen.findByRole("button", {
      name: "恢复 Compact Sticky 或拖动 Mini Tab",
    });
    const board = mini.parentElement!;
    vi.spyOn(mini, "getBoundingClientRect").mockReturnValue({
      x: 100,
      y: 200,
      left: 100,
      top: 200,
      right: 178,
      bottom: 246,
      width: 78,
      height: 46,
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
    Object.defineProperties(mini, {
      offsetWidth: { configurable: true, value: 78 },
      offsetHeight: { configurable: true, value: 46 },
    });
    dispatchPointer(mini, "pointerDown", { pointerId: 3, clientX: 110, clientY: 210 });
    dispatchPointer(mini, "pointerMove", { pointerId: 3, clientX: 140, clientY: 240 });
    dispatchPointer(mini, "pointerUp", { pointerId: 3, clientX: 140, clientY: 240 });
    expect(positionChange).toHaveBeenCalledOnce();
    expect(modeChange).not.toHaveBeenCalled();
  });

  it("offers all four Window Size Presets with Mini Tab production behavior", async () => {
    const user = userEvent.setup();
    renderHome(new MemoryStickyPort());
    await user.click(screen.getByRole("button", { name: "外观与窗口设置" }));
    for (const label of ["Sticky", "iPhone 5", "Pocket", "Book"])
      expect(screen.getByRole("button", { name: label })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "缩成 Mini Tab" })).toBeInTheDocument();
  });
});
