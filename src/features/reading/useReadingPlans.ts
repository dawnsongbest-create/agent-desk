import { useCallback, useEffect, useState } from "react";
import type { ReadingPlansPort } from "../../application/ports/reading";
import type {
  CreateReadingPlanInput,
  GenerateReadingDeliveryResult,
  ReadingPlan,
  ReadingPlanStatus,
  ReadingSessionResult,
} from "../../domain/reading";

export type ReadingPlansState = "loading" | "ready" | "saving" | "error";

export function useReadingPlans(port: ReadingPlansPort, enabled: boolean) {
  const [plans, setPlans] = useState<ReadingPlan[]>([]);
  const [state, setState] = useState<ReadingPlansState>("loading");
  const [error, setError] = useState<string | null>(null);
  const [busyPlanId, setBusyPlanId] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!enabled) return;
    setState("loading");
    try {
      setPlans(await port.listPlans());
      setState("ready");
      setError(null);
    } catch {
      setState("error");
      setError("阅读计划暂时无法加载。");
    }
  }, [enabled, port]);

  useEffect(() => {
    void load();
  }, [load]);

  const create = useCallback(
    async (input: CreateReadingPlanInput) => {
      setState("saving");
      setError(null);
      try {
        const plan = await port.createPlan(input);
        setPlans((current) => [plan, ...current]);
        setState("ready");
        return plan;
      } catch {
        setState("error");
        setError("无法创建阅读计划，请检查内容和时间。");
        return null;
      }
    },
    [port],
  );

  const setStatus = useCallback(
    async (id: string, status: ReadingPlanStatus) => {
      setBusyPlanId(id);
      setError(null);
      try {
        const plan = await port.setPlanStatus(id, status);
        setPlans((current) => current.map((item) => (item.id === id ? plan : item)));
        return plan;
      } catch {
        setError("无法更新阅读计划状态。");
        return null;
      } finally {
        setBusyPlanId(null);
      }
    },
    [port],
  );

  const generate = useCallback(
    async (id: string): Promise<GenerateReadingDeliveryResult | null> => {
      setBusyPlanId(id);
      setError(null);
      try {
        const result = await port.generateToday(id);
        setPlans((current) => current.map((item) => (item.id === id ? result.plan : item)));
        return result;
      } catch {
        setError("今天的阅读内容暂时无法生成。");
        return null;
      } finally {
        setBusyPlanId(null);
      }
    },
    [port],
  );

  const createSession = useCallback(
    (sourceDocumentId: string, content: string): Promise<ReadingSessionResult> =>
      port.createSession({ sourceDocumentId, content }),
    [port],
  );

  return {
    plans,
    state,
    error,
    busyPlanId,
    retry: load,
    create,
    setStatus,
    generate,
    createSession,
  };
}
