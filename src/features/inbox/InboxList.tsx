import type { InboxDelivery } from "../../domain/delivery";
import type { InboxLoadState } from "./useInbox";

const documentTypeLabels: Record<InboxDelivery["document"]["documentType"], string> = {
  article: "文章",
  brief: "简报",
  reading: "阅读",
  report: "报告",
};

function deliveryDate(value: string) {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(parsed);
}

export function InboxList({
  items,
  state,
  openingId,
  openError,
  onRetry,
  onOpen,
}: {
  items: InboxDelivery[];
  state: InboxLoadState;
  openingId: string | null;
  openError: string | null;
  onRetry(): void;
  onOpen(id: string): void;
}) {
  if (state === "loading" && items.length === 0) {
    return <p className="inbox-state">正在查看送来的内容…</p>;
  }
  if (state === "error" && items.length === 0) {
    return (
      <div className="inbox-state" role="alert">
        <p>收件箱暂时无法加载。</p>
        <button type="button" onClick={onRetry}>
          重试
        </button>
      </div>
    );
  }
  if (items.length === 0) {
    return (
      <div className="inbox-empty">
        <p>暂时没有新内容。</p>
        <span>送到 Agent Desk 的阅读内容会安静地出现在这里。</span>
      </div>
    );
  }
  return (
    <section className="inbox-list" aria-label="收件内容">
      {openError ? (
        <p className="inbox-open-error" role="alert">
          {openError}
        </p>
      ) : null}
      {items.map((item) => {
        const unread = item.delivery.openedAt === null;
        const source = item.document.sourceLabel ?? item.document.sourceType;
        const readingMeta =
          item.document.documentType === "reading" ? item.document.subtitle : null;
        return (
          <button
            className="inbox-item"
            type="button"
            key={item.delivery.id}
            data-unread={unread}
            disabled={openingId !== null}
            aria-label={`${unread ? "未读" : "已读"}，${item.document.title}，${source}，${documentTypeLabels[item.document.documentType]}，${deliveryDate(item.delivery.deliveredAt)}`}
            onClick={() => onOpen(item.delivery.id)}
          >
            <span className="inbox-unread-mark" aria-hidden="true" />
            <span className="inbox-item-body">
              <strong>{item.document.title}</strong>
              <span className="inbox-item-meta">
                {readingMeta ? <span className="inbox-reading-kind">今日阅读</span> : null}
                <span>{source}</span>
                <span>{readingMeta ?? documentTypeLabels[item.document.documentType]}</span>
                <time dateTime={item.delivery.deliveredAt}>
                  {deliveryDate(item.delivery.deliveredAt)}
                </time>
              </span>
            </span>
            <span className="inbox-item-state">
              {openingId === item.delivery.id ? "打开中" : unread ? "未读" : "已读"}
            </span>
          </button>
        );
      })}
    </section>
  );
}
