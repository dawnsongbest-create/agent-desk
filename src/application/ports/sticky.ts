import type { CreateStickyCardInput, StickyCard, StickyProfile } from "../../domain/sticky";

export interface StickyCardsPort {
  list(): Promise<StickyCard[]>;
  create(input: CreateStickyCardInput): Promise<StickyCard>;
  updateText(id: string, text: string): Promise<StickyCard>;
  setTaskCompleted(id: string, completed: boolean): Promise<StickyCard>;
  setTaskDueDate(id: string, dueDate: string | null): Promise<StickyCard>;
  delete(id: string): Promise<void>;
  reorder(orderedIds: string[]): Promise<StickyCard[]>;
  getProfile(): Promise<StickyProfile>;
  updateQuote(quoteText: string): Promise<StickyProfile>;
  exportRecord(id: string): Promise<boolean>;
}
