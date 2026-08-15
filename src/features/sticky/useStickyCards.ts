import { useCallback, useEffect, useRef, useState } from "react";
import type { StickyCardsPort } from "../../application/ports/sticky";
import type { CreateStickyCardInput, StickyCard, StickyProfile } from "../../domain/sticky";

export type StickyLoadState = "loading" | "ready" | "saving" | "error";

export function useStickyCards(port: StickyCardsPort) {
  const [cards, setCards] = useState<StickyCard[]>([]);
  const [state, setState] = useState<StickyLoadState>("loading");
  const [error, setError] = useState<string | null>(null);
  const [profile, setProfile] = useState<StickyProfile>({ quoteText: "", updatedAt: "" });
  const cardsRef = useRef<StickyCard[]>([]);
  const mountedRef = useRef(true);

  const publish = useCallback((next: StickyCard[]) => {
    cardsRef.current = next;
    if (mountedRef.current) setCards(next);
  }, []);

  const load = useCallback(async () => {
    setState("loading");
    setError(null);
    try {
      const [stored, storedProfile] = await Promise.all([port.list(), port.getProfile()]);
      if (!mountedRef.current) return;
      publish(stored);
      setProfile(storedProfile);
      setState("ready");
    } catch {
      if (!mountedRef.current) return;
      setState("error");
      setError("无法读取本地便利贴，请重试。");
    }
  }, [port, publish]);

  useEffect(() => {
    mountedRef.current = true;
    void load();
    return () => {
      mountedRef.current = false;
    };
  }, [load]);

  const recover = useCallback(
    async (fallback: StickyCard[]) => {
      let restored = fallback;
      try {
        restored = await port.list();
      } catch {
        // Keep the last confirmed renderer snapshot when refetch is also unavailable.
      }
      if (!mountedRef.current) return;
      publish(restored);
      setState("error");
      setError("更改未能保存，已恢复到上次确认的数据。");
    },
    [port, publish],
  );

  const create = useCallback(
    async (input: CreateStickyCardInput) => {
      const before = cardsRef.current;
      setState("saving");
      setError(null);
      try {
        const created = await port.create(input);
        if (!mountedRef.current) return false;
        publish([...before, created].sort((left, right) => left.position - right.position));
        setState("ready");
        return true;
      } catch {
        await recover(before);
        return false;
      }
    },
    [port, publish, recover],
  );

  const updateText = useCallback(
    async (id: string, text: string) => {
      const before = cardsRef.current;
      const optimistic = before.map((card) => (card.id === id ? { ...card, text } : card));
      publish(optimistic);
      setState("saving");
      setError(null);
      try {
        const updated = await port.updateText(id, text);
        if (!mountedRef.current) return false;
        publish(optimistic.map((card) => (card.id === id ? updated : card)));
        setState("ready");
        return true;
      } catch {
        await recover(before);
        return false;
      }
    },
    [port, publish, recover],
  );

  const setTaskCompleted = useCallback(
    async (id: string, completed: boolean) => {
      const before = cardsRef.current;
      const optimistic = before.map((card) => (card.id === id ? { ...card, completed } : card));
      publish(optimistic);
      setState("saving");
      setError(null);
      try {
        const updated = await port.setTaskCompleted(id, completed);
        if (!mountedRef.current) return;
        publish(optimistic.map((card) => (card.id === id ? updated : card)));
        setState("ready");
      } catch {
        await recover(before);
      }
    },
    [port, publish, recover],
  );

  const setTaskDueDate = useCallback(
    async (id: string, dueDate: string | null) => {
      const before = cardsRef.current;
      const optimistic = before.map((card) => (card.id === id ? { ...card, dueDate } : card));
      publish(optimistic);
      setState("saving");
      setError(null);
      try {
        const updated = await port.setTaskDueDate(id, dueDate);
        if (!mountedRef.current) return;
        publish(optimistic.map((card) => (card.id === id ? updated : card)));
        setState("ready");
      } catch {
        await recover(before);
      }
    },
    [port, publish, recover],
  );

  const deleteCard = useCallback(
    async (id: string) => {
      const before = cardsRef.current;
      publish(before.filter((card) => card.id !== id));
      setState("saving");
      setError(null);
      try {
        await port.delete(id);
        if (!mountedRef.current) return;
        setState("ready");
      } catch {
        await recover(before);
      }
    },
    [port, publish, recover],
  );

  const reorder = useCallback(
    async (orderedIds: string[]) => {
      const before = cardsRef.current;
      const lookup = new Map(before.map((card) => [card.id, card]));
      const optimistic = orderedIds
        .map((id, position) => {
          const card = lookup.get(id);
          return card ? { ...card, position } : null;
        })
        .filter((card): card is StickyCard => card !== null);
      if (optimistic.length !== before.length) return;

      publish(optimistic);
      setState("saving");
      setError(null);
      try {
        const stored = await port.reorder(orderedIds);
        if (!mountedRef.current) return;
        publish(stored);
        setState("ready");
      } catch {
        await recover(before);
      }
    },
    [port, publish, recover],
  );

  const updateQuote = useCallback(
    async (quoteText: string) => {
      const before = profile;
      setProfile({ ...before, quoteText });
      setState("saving");
      setError(null);
      try {
        const stored = await port.updateQuote(quoteText);
        if (!mountedRef.current) return false;
        setProfile(stored);
        setState("ready");
        return true;
      } catch {
        if (!mountedRef.current) return false;
        setProfile(before);
        setState("error");
        setError("便签一句未能保存，已恢复原文。");
        return false;
      }
    },
    [port, profile],
  );

  const exportRecord = useCallback(
    async (id: string) => {
      try {
        return await port.exportRecord(id);
      } catch {
        if (mountedRef.current) setError("导出没有完成；记录原文没有改变。");
        return false;
      }
    },
    [port],
  );

  const acceptCreated = useCallback(
    (created: StickyCard) => {
      const next = [...cardsRef.current.filter((card) => card.id !== created.id), created].sort(
        (left, right) => left.position - right.position,
      );
      publish(next);
    },
    [publish],
  );

  return {
    cards,
    profile,
    state,
    error,
    isBusy: state === "loading" || state === "saving",
    create,
    updateText,
    setTaskCompleted,
    setTaskDueDate,
    deleteCard,
    reorder,
    updateQuote,
    exportRecord,
    acceptCreated,
    retry: load,
    dismissError: () => setError(null),
  };
}
