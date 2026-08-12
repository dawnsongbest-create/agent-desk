import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { StickyCardsPort } from "../../application/ports/sticky";
import { defaultPreferences } from "../../domain/preferences";
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

function renderHome(port: StickyCardsPort, positionChange = vi.fn()) {
  return render(
    <StickyHome
      port={port}
      preferences={defaultPreferences}
      preferenceSaveState="idle"
      now={new Date("2026-08-12T12:00:00")}
      onThemeChange={vi.fn()}
      onAlwaysOnTopChange={vi.fn()}
      onWindowPresetChange={vi.fn()}
      onStickyPositionChange={positionChange}
    />,
  );
}

async function expand(user: ReturnType<typeof userEvent.setup>) {
  await user.click(await screen.findByRole("button", { name: "展开或拖动便利贴" }));
  return screen.getByRole("region", { name: "展开的便利贴" });
}

describe("StickyHome M1-B3", () => {
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
    expect(compact).toHaveTextContent("第四件");
    expect(compact).toHaveTextContent("多花点时间玩。");
    expect(compact).not.toHaveTextContent("不应出现在 Compact");
    expect(compact).not.toHaveTextContent("已完成，不应抢占");
    expect(within(compact).getByLabelText("Compact Todo 列表")).toHaveClass(
      "compact-todo-viewport",
    );
  });

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

  it("offers all four Window Size Presets without implementing Mini Tab", async () => {
    const user = userEvent.setup();
    renderHome(new MemoryStickyPort());
    await user.click(screen.getByRole("button", { name: "外观与窗口设置" }));
    for (const label of ["Sticky", "iPhone 5", "Pocket", "Book"])
      expect(screen.getByRole("button", { name: label })).toBeInTheDocument();
    expect(screen.queryByText(/Mini Tab/i)).not.toBeInTheDocument();
  });
});
