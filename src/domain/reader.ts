import type { StickyCard } from "./sticky";

export const readerDocumentTypes = ["article", "brief", "reading", "report"] as const;
export type ReaderDocumentType = (typeof readerDocumentTypes)[number];

export const readerSourceTypes = ["local", "builtin", "agent"] as const;
export type ReaderSourceType = (typeof readerSourceTypes)[number];

export type ReaderDocument = {
  id: string;
  documentType: ReaderDocumentType;
  title: string;
  subtitle: string | null;
  contentMarkdown: string;
  sourceType: ReaderSourceType;
  sourceLabel: string | null;
  createdAt: string;
  updatedAt: string;
};

export type CreateReaderDocumentInput = Pick<
  ReaderDocument,
  "documentType" | "title" | "subtitle" | "contentMarkdown" | "sourceType" | "sourceLabel"
>;

export type RecordSourceRef = {
  recordId: string;
  documentId: string;
  sourceType: "reader_selection";
  selectedText: string;
  documentTitleSnapshot: string;
  capturedAt: string;
};

export type CaptureReaderSelectionInput = {
  documentId: string;
  selectedText: string;
};

export type SelectionCaptureResult = {
  record: StickyCard;
  sourceRef: RecordSourceRef;
};
