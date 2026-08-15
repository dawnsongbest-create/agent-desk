import type {
  CaptureReaderSelectionInput,
  CreateReaderDocumentInput,
  ReaderDocument,
  SelectionCaptureResult,
} from "../../domain/reader";

export interface ReaderDocumentsPort {
  openCurrent(currentDocumentId: string | null): Promise<ReaderDocument>;
  get(id: string): Promise<ReaderDocument>;
  list(): Promise<ReaderDocument[]>;
  create(input: CreateReaderDocumentInput): Promise<ReaderDocument>;
  captureSelection(input: CaptureReaderSelectionInput): Promise<SelectionCaptureResult>;
  copyText(text: string): Promise<void>;
}
