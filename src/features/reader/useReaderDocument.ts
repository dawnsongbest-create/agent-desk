import { useCallback, useEffect, useRef, useState } from "react";
import type { ReaderDocumentsPort } from "../../application/ports/reader";
import type { ReaderDocument, SelectionCaptureResult } from "../../domain/reader";

export type ReaderLoadState = "loading" | "ready" | "error";

export function useReaderDocument(
  port: ReaderDocumentsPort,
  enabled: boolean,
  currentDocumentId: string | null,
  onCurrentDocumentChange: (id: string) => void,
) {
  const [document, setDocument] = useState<ReaderDocument | null>(null);
  const [state, setState] = useState<ReaderLoadState>("loading");
  const callbackRef = useRef(onCurrentDocumentChange);
  callbackRef.current = onCurrentDocumentChange;

  const load = useCallback(async () => {
    if (!enabled) return;
    setState("loading");
    try {
      const opened = await port.openCurrent(currentDocumentId);
      setDocument(opened);
      setState("ready");
      if (opened.id !== currentDocumentId) callbackRef.current(opened.id);
    } catch {
      setState("error");
    }
  }, [currentDocumentId, enabled, port]);

  useEffect(() => {
    let active = true;
    if (!enabled) return;
    setState("loading");
    void port
      .openCurrent(currentDocumentId)
      .then((opened) => {
        if (!active) return;
        setDocument(opened);
        setState("ready");
        if (opened.id !== currentDocumentId) callbackRef.current(opened.id);
      })
      .catch(() => {
        if (active) setState("error");
      });
    return () => {
      active = false;
    };
  }, [currentDocumentId, enabled, port]);

  const captureSelection = useCallback(
    (documentId: string, selectedText: string): Promise<SelectionCaptureResult> =>
      port.captureSelection({ documentId, selectedText }),
    [port],
  );

  return { document, state, retry: load, captureSelection, copyText: port.copyText };
}
