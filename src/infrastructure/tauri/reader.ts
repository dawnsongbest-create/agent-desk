import { invoke } from "@tauri-apps/api/core";
import type { ReaderDocumentsPort } from "../../application/ports/reader";
import type {
  CaptureReaderSelectionInput,
  CreateReaderDocumentInput,
  ReaderDocument,
  SelectionCaptureResult,
} from "../../domain/reader";

export const tauriReaderDocuments: ReaderDocumentsPort = {
  openCurrent(currentDocumentId) {
    return invoke<ReaderDocument>("open_reader_document", { input: { currentDocumentId } });
  },
  get(id) {
    return invoke<ReaderDocument>("get_reader_document", { input: { id } });
  },
  list() {
    return invoke<ReaderDocument[]>("list_reader_documents");
  },
  create(input: CreateReaderDocumentInput) {
    return invoke<ReaderDocument>("create_reader_document", { input });
  },
  captureSelection(input: CaptureReaderSelectionInput) {
    return invoke<SelectionCaptureResult>("capture_reader_selection", { input });
  },
  copyText(text) {
    if (!navigator.clipboard?.writeText) {
      return Promise.reject(new Error("The WebView clipboard API is unavailable."));
    }
    return navigator.clipboard.writeText(text);
  },
};
