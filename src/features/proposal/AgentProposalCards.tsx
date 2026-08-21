import type { AgentProposal } from "../../domain/proposal";

const typeLabels: Record<AgentProposal["type"], string> = {
  todo: "待办建议",
  record: "记录建议",
  reading: "阅读建议",
};

export function AgentProposalCards({
  proposals,
  busyId,
  errorId,
  onAccept,
  onReject,
}: {
  proposals: AgentProposal[];
  busyId: string | null;
  errorId: string | null;
  onAccept(id: string): void;
  onReject(id: string): void;
}) {
  const pending = proposals.filter((proposal) => proposal.status === "pending");
  if (pending.length === 0) return null;
  return (
    <aside className="agent-proposal-stack" aria-label="Agent 建议">
      <header>
        <span>Agent 建议</span>
        <small>由你决定是否加入工作区</small>
      </header>
      {pending.map((proposal) => {
        const busy = busyId === proposal.id;
        return (
          <article
            className="agent-proposal-card"
            key={proposal.id}
            data-proposal-type={proposal.type}
          >
            <p className="agent-proposal-kind">{typeLabels[proposal.type]}</p>
            <h2>{proposal.title}</h2>
            <p>{proposal.description}</p>
            <footer>
              <button type="button" disabled={busy} onClick={() => onReject(proposal.id)}>
                忽略
              </button>
              <button
                className="agent-proposal-accept"
                type="button"
                disabled={busy}
                onClick={() => onAccept(proposal.id)}
              >
                {busy
                  ? "处理中…"
                  : proposal.type === "todo"
                    ? "加入待办"
                    : proposal.type === "record"
                      ? "保存到记录"
                      : "加入今日阅读"}
              </button>
            </footer>
            {errorId === proposal.id ? (
              <p className="agent-proposal-error" role="alert">
                操作未完成，请重试。
              </p>
            ) : null}
          </article>
        );
      })}
    </aside>
  );
}
