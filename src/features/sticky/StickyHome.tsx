import type { StickyCardsPort } from "../../application/ports/sticky";
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

type StickyHomeProps = {
  port: StickyCardsPort;
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
};

export function StickyHome({
  port,
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
}: StickyHomeProps) {
  const sticky = useStickyCards(port);

  return (
    <StickyShell
      preferences={preferences}
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
