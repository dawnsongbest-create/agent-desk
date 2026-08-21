import type { ReaderDocument } from "./reader";
import type { ReadingSession } from "./reading";
import type { StickyCard } from "./sticky";

export type AgentProposalType = "todo" | "record" | "reading";
export type AgentProposalStatus = "pending" | "accepted" | "rejected";

export type AgentProposal = {
  id: string;
  type: AgentProposalType;
  title: string;
  description: string;
  payloadJson: string;
  sourceDeliveryId: string | null;
  status: AgentProposalStatus;
  createdAt: string;
  resolvedAt: string | null;
};

export type AcceptProposalResult = {
  proposal: AgentProposal;
  card: StickyCard | null;
  readingSession: { session: ReadingSession; document: ReaderDocument } | null;
};
