import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import type { DeliveriesPort } from "../../application/ports/delivery";
import type { InboxDelivery } from "../../domain/delivery";

export type InboxLoadState = "loading" | "ready" | "error";

export function useInbox(port: DeliveriesPort, enabled: boolean) {
  const [items, setItems] = useState<InboxDelivery[]>([]);
  const itemsRef = useRef<InboxDelivery[]>([]);
  const loadSequence = useRef({ value: 0 });
  const [unreadCount, setUnreadCount] = useState(0);
  const [state, setState] = useState<InboxLoadState>("loading");
  const [openingId, setOpeningId] = useState<string | null>(null);
  const [openError, setOpenError] = useState<string | null>(null);

  const load = useCallback(
    async (background = false) => {
      if (!enabled) return;
      const sequence = ++loadSequence.current.value;
      if (!background) setState("loading");
      try {
        const [loadedItems, count] = await Promise.all([port.listInbox(), port.getUnreadCount()]);
        if (sequence !== loadSequence.current.value) return;
        itemsRef.current = loadedItems;
        setItems(loadedItems);
        setUnreadCount(count);
        setState("ready");
        if (!background) setOpenError(null);
      } catch {
        if (sequence === loadSequence.current.value) {
          setState((previous) => (background && previous === "ready" ? previous : "error"));
        }
      }
    },
    [enabled, port],
  );

  useEffect(() => {
    if (!enabled) return;
    const generation = loadSequence.current;
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    void load();
    void listen("agent-desk://inbox-changed", () => {
      if (!cancelled) void load(true);
    })
      .then((dispose) => {
        if (cancelled) dispose();
        else unlisten = dispose;
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
      ++generation.value;
      unlisten?.();
    };
  }, [enabled, load]);

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
        // A concurrent invalidation can have read the pre-open snapshot.
        void load(true);
        return opened;
      } catch {
        setOpenError("无法打开这份内容，请重试。");
        return null;
      } finally {
        setOpeningId(null);
      }
    },
    [load, port],
  );

  return { items, unreadCount, state, openingId, openError, retry: load, open };
}
