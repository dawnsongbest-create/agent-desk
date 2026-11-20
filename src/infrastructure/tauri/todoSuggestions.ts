import { invoke } from "@tauri-apps/api/core";
import type {
  TodoSuggestionBatch,
  TodoSuggestionsPort,
} from "../../application/ports/todoSuggestions";

export const tauriTodoSuggestions: TodoSuggestionsPort = {
  listPending: () => invoke<TodoSuggestionBatch[]>("list_pending_todo_batches"),
  accept: (batchId) => invoke<{ todoIds: string[] }>("accept_todo_batch", { input: { batchId } }),
  reject: (batchId) => invoke<void>("reject_todo_batch", { input: { batchId } }),
};
