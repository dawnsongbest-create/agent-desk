import type { StickyCardsPort } from "../../application/ports/sticky";
import type { ReaderDocumentsPort } from "../../application/ports/reader";
import type { DeliveriesPort } from "../../application/ports/delivery";
import type { ReadingPlansPort } from "../../application/ports/reading";
import type { AgentProposalsPort } from "../../application/ports/proposal";
import type { CreateReadingPlanInput, ReadingPlanStatus } from "../../domain/reading";
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
import { useInbox } from "../inbox/useInbox";
import { useReadingPlans } from "../reading/useReadingPlans";
import { useAgentProposals } from "../proposal/useAgentProposals";

type StickyHomeProps = {
  port: StickyCardsPort;
  readerPort: ReaderDocumentsPort;
  deliveryPort: DeliveriesPort;
  readingPort: ReadingPlansPort;
  proposalPort: AgentProposalsPort;
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
  deliveryPort,
  readingPort,
  proposalPort,
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
  const inbox = useInbox(deliveryPort, preferenceSaveState !== "loading");
  const reading = useReadingPlans(readingPort, preferenceSaveState !== "loading");
  const proposals = useAgentProposals(proposalPort, reader.document?.id ?? null);

  async function captureSelection(documentId: string, text: string) {
    try {
      const captured = await reader.captureSelection(documentId, text);
      sticky.acceptCreated(captured.record);
      return true;
    } catch {
      return false;
    }
  }

  async function openDelivery(id: string) {
    const opened = await inbox.open(id);
    if (!opened) return false;
    reader.acceptOpened(opened.document);
    onCurrentReaderDocumentChange(opened.document.id);
    onReaderContentVisibilityChange(true);
    return true;
  }

  async function createReadingPlan(input: CreateReadingPlanInput) {
    return (await reading.create(input)) !== null;
  }

  async function generateReadingDelivery(id: string) {
    const generated = await reading.generate(id);
    if (!generated) return false;
    await inbox.retry();
    return true;
  }

  async function setReadingPlanStatus(id: string, status: ReadingPlanStatus) {
    await reading.setStatus(id, status);
  }

  async function createReadingSession(documentId: string, text: string) {
    try {
      const created = await reading.createSession(documentId, text);
      reader.acceptOpened(created.document);
      onCurrentReaderDocumentChange(created.document.id);
      onReaderContentVisibilityChange(true);
      return true;
    } catch {
      return false;
    }
  }

  async function acceptProposal(id: string) {
    const accepted = await proposals.accept(id);
    if (!accepted) return false;
    if (accepted.card) sticky.acceptCreated(accepted.card);
    if (accepted.readingSession) {
      reader.acceptOpened(accepted.readingSession.document);
      onCurrentReaderDocumentChange(accepted.readingSession.document.id);
      onReaderContentVisibilityChange(true);
    }
    return true;
  }

  return (
    <StickyShell
      preferences={preferences}
      readerDocument={reader.document}
      readerState={reader.state}
      inboxItems={inbox.items}
      inboxUnreadCount={inbox.unreadCount}
      inboxState={inbox.state}
      inboxOpeningId={inbox.openingId}
      inboxOpenError={inbox.openError}
      readingPlans={reading.plans}
      readingPlansState={reading.state}
      readingPlansError={reading.error}
      readingBusyPlanId={reading.busyPlanId}
      agentProposals={proposals.proposals}
      proposalBusyId={proposals.busyId}
      proposalErrorId={proposals.errorId}
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
      onRetryInbox={inbox.retry}
      onOpenDelivery={openDelivery}
      onCopyReaderSelection={reader.copyText}
      onCaptureReaderSelection={captureSelection}
      onCreateReadingSession={createReadingSession}
      onRetryReadingPlans={reading.retry}
      onCreateReadingPlan={createReadingPlan}
      onGenerateReadingDelivery={generateReadingDelivery}
      onSetReadingPlanStatus={setReadingPlanStatus}
      onAcceptProposal={acceptProposal}
      onRejectProposal={proposals.reject}
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
