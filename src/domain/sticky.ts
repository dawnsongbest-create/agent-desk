export type StickyCardKind = "note" | "task";

export type StickyCard = {
  id: string;
  kind: StickyCardKind;
  text: string;
  completed: boolean;
  dueDate: string | null;
  position: number;
  createdAt: string;
  updatedAt: string;
};

export type CreateStickyCardInput = {
  kind: StickyCardKind;
  text: string;
  dueDate: string | null;
};

export function parseCapture(
  text: string,
  selectedKind: StickyCardKind,
  dueDate: string | null,
): CreateStickyCardInput | null {
  const normalized = text.trim();
  if (!normalized) return null;

  const quickTask = normalized.match(/^\[\s*\]\s+([\s\S]+)$/);
  if (quickTask) {
    return {
      kind: "task",
      text: quickTask[1].trim(),
      dueDate,
    };
  }

  return {
    kind: selectedKind,
    text: normalized,
    dueDate: selectedKind === "task" ? dueDate : null,
  };
}

export function reorderCardIds(cards: StickyCard[], activeId: string, overId: string) {
  const from = cards.findIndex((card) => card.id === activeId);
  const to = cards.findIndex((card) => card.id === overId);
  if (from < 0 || to < 0 || from === to) {
    return cards.map((card) => card.id);
  }

  const ids = cards.map((card) => card.id);
  const [moved] = ids.splice(from, 1);
  ids.splice(to, 0, moved);
  return ids;
}

export function formatLocalDueDate(value: string, locale?: string) {
  const [year, month, day] = value.split("-").map(Number);
  return new Intl.DateTimeFormat(locale, {
    month: "short",
    day: "numeric",
  }).format(new Date(year, month - 1, day));
}
