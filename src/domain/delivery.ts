import type { ReaderDocument, ReaderDocumentType, ReaderSourceType } from "./reader";

export type CreateDeliveryInput = {
  idempotencyKey: string;
  documentType: ReaderDocumentType;
  title: string;
  subtitle: string | null;
  contentMarkdown: string;
  sourceType: ReaderSourceType;
  sourceLabel: string | null;
  deliveredAt: string | null;
};

export type Delivery = {
  id: string;
  documentId: string;
  idempotencyKey: string;
  deliveredAt: string;
  openedAt: string | null;
};

export type InboxDelivery = {
  delivery: Delivery;
  document: ReaderDocument;
};

export type IngestDeliveryResult = {
  item: InboxDelivery;
  created: boolean;
};
