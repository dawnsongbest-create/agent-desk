import { invoke } from "@tauri-apps/api/core";
import type { ReadingPlansPort } from "../../application/ports/reading";
import type {
  CreateReadingPlanInput,
  CreateReadingSessionInput,
  GenerateReadingDeliveryResult,
  ReadingPlan,
  ReadingPlanStatus,
  ReadingSessionResult,
} from "../../domain/reading";

export const tauriReadingPlans: ReadingPlansPort = {
  createPlan(input: CreateReadingPlanInput) {
    return invoke<ReadingPlan>("create_reading_plan", { input });
  },
  listPlans() {
    return invoke<ReadingPlan[]>("list_reading_plans");
  },
  setPlanStatus(id: string, status: ReadingPlanStatus) {
    return invoke<ReadingPlan>("set_reading_plan_status", { input: { id, status } });
  },
  generateToday(id: string) {
    return invoke<GenerateReadingDeliveryResult>("generate_reading_delivery", { input: { id } });
  },
  createSession(input: CreateReadingSessionInput) {
    return invoke<ReadingSessionResult>("create_reading_session", { input });
  },
};
