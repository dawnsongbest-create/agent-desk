import { invoke } from "@tauri-apps/api/core";
import type { DeliveriesPort } from "../../application/ports/delivery";
import type {
  CreateDeliveryInput,
  InboxDelivery,
  IngestDeliveryResult,
} from "../../domain/delivery";

export const tauriDeliveries: DeliveriesPort = {
  ingest(input: CreateDeliveryInput) {
    return invoke<IngestDeliveryResult>("ingest_delivery", { input });
  },
  listInbox() {
    return invoke<InboxDelivery[]>("list_inbox");
  },
  getUnreadCount() {
    return invoke<number>("get_inbox_unread_count");
  },
  open(id: string) {
    return invoke<InboxDelivery>("open_delivery", { input: { id } });
  },
};
