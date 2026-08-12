import { describe, expect, it } from "vitest";
import {
  parseCapture,
  recordDisplayTitle,
  recordExcerpt,
  reorderCardIds,
  reorderCardIdsWithinKind,
  type StickyCard,
} from "./sticky";

function card(id: string, position: number): StickyCard {
  return {
    id,
    kind: "task",
    text: id,
    completed: false,
    dueDate: null,
    position,
    createdAt: "2026-08-10T00:00:00Z",
    updatedAt: "2026-08-10T00:00:00Z",
  };
}

function kindCard(id: string, kind: "note" | "task", position: number): StickyCard {
  return { ...card(id, position), kind };
}

describe("Sticky domain helpers", () => {
  it("recognizes quick Task syntax without requiring it for normal capture", () => {
    expect(parseCapture("[ ] 写 PRD", "note", null)).toEqual({
      kind: "task",
      text: "写 PRD",
      dueDate: null,
    });
    expect(parseCapture("一个普通想法", "note", "2026-08-12")).toEqual({
      kind: "note",
      text: "一个普通想法",
      dueDate: null,
    });
  });

  it("computes a unified Note/Task placement order without mutating cards", () => {
    const cards = [card("first", 0), card("second", 1), card("third", 2)];
    expect(reorderCardIds(cards, "first", "third")).toEqual(["second", "third", "first"]);
    expect(cards.map((item) => item.id)).toEqual(["first", "second", "third"]);
  });

  it("does not create a reorder for cancelled or unknown targets", () => {
    const cards = [card("first", 0), card("second", 1)];
    expect(reorderCardIds(cards, "first", "first")).toEqual(["first", "second"]);
    expect(reorderCardIds(cards, "first", "missing")).toEqual(["first", "second"]);
  });

  it("reorders one face while preserving the other kind's unified placement slots", () => {
    const cards = [
      kindCard("note-1", "note", 0),
      kindCard("task-1", "task", 1),
      kindCard("note-2", "note", 2),
      kindCard("task-2", "task", 3),
    ];

    expect(reorderCardIdsWithinKind(cards, "task", "task-2", "task-1")).toEqual([
      "note-1",
      "task-2",
      "note-2",
      "task-1",
    ]);
  });

  it("derives a Record title and excerpt without changing the source text", () => {
    const source = "\n  第一行标题  \n第二段有很多内容";
    expect(recordDisplayTitle(source)).toBe("第一行标题");
    expect(recordExcerpt(source, 12)).toBe("第一行标题 第二段有很多…");
    expect(source).toBe("\n  第一行标题  \n第二段有很多内容");
  });
});
