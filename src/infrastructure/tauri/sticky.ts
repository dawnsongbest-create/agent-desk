import { invoke } from "@tauri-apps/api/core";
import type { StickyCardsPort } from "../../application/ports/sticky";
import type { CreateStickyCardInput, StickyCard } from "../../domain/sticky";

export const tauriStickyCards: StickyCardsPort = {
  list() {
    return invoke<StickyCard[]>("list_sticky_cards");
  },
  create(input: CreateStickyCardInput) {
    return invoke<StickyCard>("create_sticky_card", { input });
  },
  updateText(id: string, text: string) {
    return invoke<StickyCard>("update_sticky_text", { input: { id, text } });
  },
  setTaskCompleted(id: string, completed: boolean) {
    return invoke<StickyCard>("set_task_completed", { input: { id, completed } });
  },
  setTaskDueDate(id: string, dueDate: string | null) {
    return invoke<StickyCard>("set_task_due_date", { input: { id, dueDate } });
  },
  delete(id: string) {
    return invoke<void>("delete_sticky_card", { input: { id } });
  },
  reorder(orderedIds: string[]) {
    return invoke<StickyCard[]>("reorder_sticky_cards", { input: { orderedIds } });
  },
};
