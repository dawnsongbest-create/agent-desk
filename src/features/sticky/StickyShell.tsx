import type { Preferences, ThemeMode } from "../../domain/preferences";
import { themeModes } from "../../domain/preferences";

type SaveState = "loading" | "idle" | "saving" | "saved" | "error";

type StickyShellProps = {
  preferences: Preferences;
  saveState: SaveState;
  now?: Date;
  onThemeChange(theme: ThemeMode): void;
  onAlwaysOnTopChange(alwaysOnTop: boolean): void;
};

const themeLabels: Record<ThemeMode, string> = {
  system: "System",
  light: "Light",
  dark: "Dark",
};

function statusLabel(state: SaveState) {
  switch (state) {
    case "loading":
      return "Loading preferences…";
    case "saving":
      return "Saving…";
    case "saved":
      return "Saved locally";
    case "error":
      return "Preference unavailable";
    default:
      return "Local-first foundation";
  }
}

export function StickyShell({
  preferences,
  saveState,
  now = new Date(),
  onThemeChange,
  onAlwaysOnTopChange,
}: StickyShellProps) {
  const date = new Intl.DateTimeFormat(undefined, {
    weekday: "long",
    month: "short",
    day: "numeric",
  }).format(now);

  return (
    <main className="sticky-shell" aria-label="Agent Desk Sticky Home">
      <header className="sticky-header">
        <div>
          <p className="eyebrow">Agent Desk</p>
          <h1 className="sticky-date">{date}</h1>
        </div>
        <span className="foundation-badge">M1-A</span>
      </header>

      <section className="sticky-preview" aria-labelledby="foundation-title">
        <div className="preview-rule" />
        <h2 className="preview-title" id="foundation-title">
          Your quiet desktop surface.
        </h2>
        <p className="preview-copy">
          The native shell, local persistence, and preferences are ready. Capture arrives in the
          next approved gate.
        </p>
      </section>

      <footer className="shell-controls">
        <div className="control-row">
          <span className="control-label" id="theme-label">
            Appearance
          </span>
          <div className="theme-switcher" role="group" aria-labelledby="theme-label">
            {themeModes.map((theme) => (
              <button
                className="theme-button"
                type="button"
                key={theme}
                disabled={saveState === "loading"}
                aria-pressed={preferences.theme === theme}
                onClick={() => onThemeChange(theme)}
              >
                {themeLabels[theme]}
              </button>
            ))}
          </div>
        </div>

        <div className="control-row">
          <span className="save-status" data-state={saveState} aria-live="polite">
            {statusLabel(saveState)}
          </span>
          <button
            className="pin-button"
            type="button"
            disabled={saveState === "loading"}
            aria-pressed={preferences.alwaysOnTop}
            onClick={() => onAlwaysOnTopChange(!preferences.alwaysOnTop)}
          >
            {preferences.alwaysOnTop ? "Pinned" : "Pin window"}
          </button>
        </div>
      </footer>
    </main>
  );
}
