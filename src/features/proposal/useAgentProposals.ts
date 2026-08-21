import { useCallback, useEffect, useState } from "react";
import type { AgentProposalsPort } from "../../application/ports/proposal";
import type { AcceptProposalResult, AgentProposal } from "../../domain/proposal";

export type AgentProposalLoadState = "idle" | "loading" | "ready" | "error";

export function useAgentProposals(port: AgentProposalsPort, documentId: string | null) {
  const [proposals, setProposals] = useState<AgentProposal[]>([]);
  const [state, setState] = useState<AgentProposalLoadState>("idle");
  const [busyId, setBusyId] = useState<string | null>(null);
  const [errorId, setErrorId] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!documentId) {
      setProposals([]);
      setState("idle");
      return;
    }
    setState("loading");
    try {
      setProposals(await port.listForDocument(documentId));
      setState("ready");
      setErrorId(null);
    } catch {
      setProposals([]);
      setState("error");
    }
  }, [documentId, port]);

  useEffect(() => {
    void load();
  }, [load]);

  const accept = useCallback(
    async (id: string): Promise<AcceptProposalResult | null> => {
      setBusyId(id);
      setErrorId(null);
      try {
        const result = await port.accept(id);
        setProposals((current) =>
          current.map((proposal) => (proposal.id === id ? result.proposal : proposal)),
        );
        return result;
      } catch {
        setErrorId(id);
        return null;
      } finally {
        setBusyId(null);
      }
    },
    [port],
  );

  const reject = useCallback(
    async (id: string) => {
      setBusyId(id);
      setErrorId(null);
      try {
        const proposal = await port.reject(id);
        setProposals((current) => current.map((item) => (item.id === id ? proposal : item)));
        return true;
      } catch {
        setErrorId(id);
        return false;
      } finally {
        setBusyId(null);
      }
    },
    [port],
  );

  return { proposals, state, busyId, errorId, retry: load, accept, reject };
}
