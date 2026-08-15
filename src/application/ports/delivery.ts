import type {
  CreateDeliveryInput,
  InboxDelivery,
  IngestDeliveryResult,
} from "../../domain/delivery";

export interface DeliveriesPort {
  ingest(input: CreateDeliveryInput): Promise<IngestDeliveryResult>;
  listInbox(): Promise<InboxDelivery[]>;
  getUnreadCount(): Promise<number>;
  open(id: string): Promise<InboxDelivery>;
}
