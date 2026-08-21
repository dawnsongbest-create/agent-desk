import { invoke } from "@tauri-apps/api/core";
import type { AgentProposalsPort } from "../../application/ports/proposal";
import type { AcceptProposalResult, AgentProposal } from "../../domain/proposal";

export const tauriAgentProposals: AgentProposalsPort = {
  listForDocument(documentId: string) {
    return invoke<AgentProposal[]>("list_agent_proposals", { input: { documentId } });
  },
  accept(id: string) {
    return invoke<AcceptProposalResult>("accept_agent_proposal", { input: { id } });
  },
  reject(id: string) {
    return invoke<AgentProposal>("reject_agent_proposal", { input: { id } });
  },
};
