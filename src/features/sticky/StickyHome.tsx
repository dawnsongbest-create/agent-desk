import type { StickyCardsPort } from "../../application/ports/sticky";
import type { ReaderDocumentsPort } from "../../application/ports/reader";
import type {
  Preferences,
  ReaderFontSize,
  ReaderLineSpacing,
  StickyMode,
  StickyPosition,
  ThemeMode,
  WindowPreset,
} from "../../domain/preferences";
import { StickyShell } from "./StickyShell";
import { useStickyCards } from "./useStickyCards";
import { useReaderDocument } from "../reader/useReaderDocument";

type StickyHomeProps = {
  port: StickyCardsPort;
  readerPort: ReaderDocumentsPort;
  preferences: Preferences;
  preferenceSaveState: "loading" | "idle" | "saving" | "saved" | "error";
  now?: Date;
  onThemeChange(theme: ThemeMode): void;
  onAlwaysOnTopChange(alwaysOnTop: boolean): void;
  onWindowPresetChange(windowPreset: WindowPreset): void;
  onStickyPositionChange(position: StickyPosition): void;
  onStickyModeChange(mode: StickyMode): void;
  onReaderFontSizeChange(size: ReaderFontSize): void;
  onReaderLineSpacingChange(spacing: ReaderLineSpacing): void;
  onReaderContentVisibilityChange(visible: boolean): void;
  onCurrentReaderDocumentChange(id: string): void;
};

export function StickyHome({
  port,
  readerPort,
  preferences,
  preferenceSaveState,
  now,
  onThemeChange,
  onAlwaysOnTopChange,
  onWindowPresetChange,
  onStickyPositionChange,
  onStickyModeChange,
  onReaderFontSizeChange,
  onReaderLineSpacingChange,
  onReaderContentVisibilityChange,
  onCurrentReaderDocumentChange,
}: StickyHomeProps) {
  const sticky = useStickyCards(port);
  const reader = useReaderDocument(
    readerPort,
    preferenceSaveState !== "loading",
    preferences.currentReaderDocumentId,
    onCurrentReaderDocumentChange,
  );

  async function captureSelection(documentId: string, text: string) {
    try {
      const captured = await reader.captureSelection(documentId, text);
      sticky.acceptCreated(captured.record);
      return true;
    } catch {
      return false;
    }
  }

  return (
    <StickyShell
      preferences={preferences}
      readerDocument={reader.document}
      readerState={reader.state}
      preferenceSaveState={preferenceSaveState}
      cards={sticky.cards}
      profile={sticky.profile}
      stickyState={sticky.state}
      error={sticky.error}
      now={now}
      onThemeChange={onThemeChange}
      onAlwaysOnTopChange={onAlwaysOnTopChange}
      onWindowPresetChange={onWindowPresetChange}
      onStickyPositionChange={onStickyPositionChange}
      onStickyModeChange={onStickyModeChange}
      onReaderFontSizeChange={onReaderFontSizeChange}
      onReaderLineSpacingChange={onReaderLineSpacingChange}
      onReaderContentVisibilityChange={onReaderContentVisibilityChange}
      onRetryReader={reader.retry}
      onCopyReaderSelection={reader.copyText}
      onCaptureReaderSelection={captureSelection}
      onCreate={sticky.create}
      onUpdateText={sticky.updateText}
      onTaskCompleted={sticky.setTaskCompleted}
      onTaskDueDate={sticky.setTaskDueDate}
      onDelete={sticky.deleteCard}
      onReorder={sticky.reorder}
      onUpdateQuote={sticky.updateQuote}
      onExportRecord={sticky.exportRecord}
      onRetry={sticky.retry}
      onDismissError={sticky.dismissError}
    />
  );
}
