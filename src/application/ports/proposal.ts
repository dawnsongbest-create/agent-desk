import type { AcceptProposalResult, AgentProposal } from "../../domain/proposal";

export interface AgentProposalsPort {
  listForDocument(documentId: string): Promise<AgentProposal[]>;
  accept(id: string): Promise<AcceptProposalResult>;
  reject(id: string): Promise<AgentProposal>;
}
