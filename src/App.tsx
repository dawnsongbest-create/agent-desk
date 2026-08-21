import { useEffect, useRef, useState } from "react";
import type { Preferences } from "./domain/preferences";
import { defaultPreferences } from "./domain/preferences";
import { StickyHome } from "./features/sticky/StickyHome";
import { tauriPreferences } from "./infrastructure/tauri/preferences";
import { tauriDeliveries } from "./infrastructure/tauri/delivery";
import { tauriReaderDocuments } from "./infrastructure/tauri/reader";
import { tauriReadingPlans } from "./infrastructure/tauri/reading";
import { tauriStickyCards } from "./infrastructure/tauri/sticky";
import { applyTheme } from "./styles/theme";
import "./App.css";

type SaveState = "loading" | "idle" | "saving" | "saved" | "error";

function App() {
  const [preferences, setPreferences] = useState<Preferences>(defaultPreferences);
  const [saveState, setSaveState] = useState<SaveState>("loading");
  const preferencesRef = useRef(defaultPreferences);
  const latestSave = useRef(0);
  const saveQueue = useRef<Promise<void>>(Promise.resolve());

  useEffect(() => {
    let cancelled = false;

    tauriPreferences
      .load()
      .then((stored) => {
        if (cancelled) return;
        preferencesRef.current = stored;
        setPreferences(stored);
        applyTheme(stored.theme);
        setSaveState("idle");
      })
      .catch(() => {
        if (!cancelled) setSaveState("error");
      });

    return () => {
      cancelled = true;
    };
  }, []);

  function save(next: Preferences) {
    preferencesRef.current = next;
    setPreferences(next);
    applyTheme(next.theme);
    setSaveState("saving");
    const saveSequence = ++latestSave.current;

    saveQueue.current = saveQueue.current
      .catch(() => undefined)
      .then(() => tauriPreferences.save(next))
      .then((stored) => {
        if (saveSequence !== latestSave.current) return;
        preferencesRef.current = stored;
        setPreferences(stored);
        applyTheme(stored.theme);
        setSaveState("saved");
      })
      .catch(() => {
        if (saveSequence === latestSave.current) setSaveState("error");
      });
  }

  return (
    <StickyHome
      port={tauriStickyCards}
      readerPort={tauriReaderDocuments}
      deliveryPort={tauriDeliveries}
      readingPort={tauriReadingPlans}
      preferences={preferences}
      preferenceSaveState={saveState}
      onThemeChange={(theme) => save({ ...preferencesRef.current, theme })}
      onAlwaysOnTopChange={(alwaysOnTop) => save({ ...preferencesRef.current, alwaysOnTop })}
      onWindowPresetChange={(windowPreset) => {
        void tauriPreferences
          .applyWindowPreset(windowPreset)
          .then(() => save({ ...preferencesRef.current, windowPreset }))
          .catch(() => setSaveState("error"));
      }}
      onStickyPositionChange={(stickyPosition) =>
        save({ ...preferencesRef.current, stickyPosition })
      }
      onStickyModeChange={(stickyMode) => save({ ...preferencesRef.current, stickyMode })}
      onReaderFontSizeChange={(readerFontSize) =>
        save({ ...preferencesRef.current, readerFontSize })
      }
      onReaderLineSpacingChange={(readerLineSpacing) =>
        save({ ...preferencesRef.current, readerLineSpacing })
      }
      onReaderContentVisibilityChange={(readerContentVisible) =>
        save({ ...preferencesRef.current, readerContentVisible })
      }
      onCurrentReaderDocumentChange={(currentReaderDocumentId) =>
        save({ ...preferencesRef.current, currentReaderDocumentId })
      }
    />
  );
}

export default App;
