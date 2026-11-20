export type TodoSuggestionItem = {
  text: string;
  priority: "high" | "medium" | "low";
  due_date?: string;
  reason?: string;
};

export type TodoSuggestionBatch = {
  id: string;
  source: "codex" | "dumate";
  items: TodoSuggestionItem[];
  status: "pending" | "accepted" | "rejected";
  createdAt: string;
};

export type TodoSuggestionsPort = {
  listPending(): Promise<TodoSuggestionBatch[]>;
  accept(batchId: string): Promise<{ todoIds: string[] }>;
  reject(batchId: string): Promise<void>;
};

export const TODO_SUGGESTIONS_CHANGED = "agent-desk://todo-suggestions-changed";
export const TODO_CARDS_CHANGED = "agent-desk://todo-cards-changed";
