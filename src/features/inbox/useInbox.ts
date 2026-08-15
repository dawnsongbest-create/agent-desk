import { useCallback, useEffect, useRef, useState } from "react";
import type { DeliveriesPort } from "../../application/ports/delivery";
import type { InboxDelivery } from "../../domain/delivery";

export type InboxLoadState = "loading" | "ready" | "error";

export function useInbox(port: DeliveriesPort, enabled: boolean) {
  const [items, setItems] = useState<InboxDelivery[]>([]);
  const itemsRef = useRef<InboxDelivery[]>([]);
  const [unreadCount, setUnreadCount] = useState(0);
  const [state, setState] = useState<InboxLoadState>("loading");
  const [openingId, setOpeningId] = useState<string | null>(null);
  const [openError, setOpenError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!enabled) return;
    setState("loading");
    try {
      const [loadedItems, count] = await Promise.all([port.listInbox(), port.getUnreadCount()]);
      itemsRef.current = loadedItems;
      setItems(loadedItems);
      setUnreadCount(count);
      setState("ready");
      setOpenError(null);
    } catch {
      setState("error");
    }
  }, [enabled, port]);

  useEffect(() => {
    void load();
  }, [load]);

  const open = useCallback(
    async (id: string) => {
      setOpeningId(id);
      setOpenError(null);
      try {
        const opened = await port.open(id);
        const wasUnread = itemsRef.current.some(
          (item) => item.delivery.id === id && item.delivery.openedAt === null,
        );
        const nextItems = itemsRef.current.map((item) => (item.delivery.id === id ? opened : item));
        itemsRef.current = nextItems;
        setItems(nextItems);
        if (opened.delivery.openedAt) {
          setUnreadCount((current) => (wasUnread ? Math.max(0, current - 1) : current));
        }
        return opened;
      } catch {
        setOpenError("无法打开这份内容，请重试。");
        return null;
      } finally {
        setOpeningId(null);
      }
    },
    [port],
  );

  return { items, unreadCount, state, openingId, openError, retry: load, open };
}
