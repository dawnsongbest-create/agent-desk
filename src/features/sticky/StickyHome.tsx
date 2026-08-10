import type { StickyCardsPort } from "../../application/ports/sticky";
import type { Preferences, ThemeMode } from "../../domain/preferences";
import { StickyShell } from "./StickyShell";
import { useStickyCards } from "./useStickyCards";

type StickyHomeProps = {
  port: StickyCardsPort;
  preferences: Preferences;
  preferenceSaveState: "loading" | "idle" | "saving" | "saved" | "error";
  now?: Date;
  onThemeChange(theme: ThemeMode): void;
  onAlwaysOnTopChange(alwaysOnTop: boolean): void;
};

export function StickyHome({
  port,
  preferences,
  preferenceSaveState,
  now,
  onThemeChange,
  onAlwaysOnTopChange,
}: StickyHomeProps) {
  const sticky = useStickyCards(port);

  return (
    <StickyShell
      preferences={preferences}
      preferenceSaveState={preferenceSaveState}
      cards={sticky.cards}
      stickyState={sticky.state}
      error={sticky.error}
      now={now}
      onThemeChange={onThemeChange}
      onAlwaysOnTopChange={onAlwaysOnTopChange}
      onCreate={sticky.create}
      onUpdateText={sticky.updateText}
      onTaskCompleted={sticky.setTaskCompleted}
      onTaskDueDate={sticky.setTaskDueDate}
      onDelete={sticky.deleteCard}
      onReorder={sticky.reorder}
      onRetry={sticky.retry}
      onDismissError={sticky.dismissError}
    />
  );
}
