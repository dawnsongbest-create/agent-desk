import type {
  CreateReadingPlanInput,
  CreateReadingSessionInput,
  GenerateReadingDeliveryResult,
  ReadingPlan,
  ReadingPlanStatus,
  ReadingSessionResult,
} from "../../domain/reading";

export interface ReadingPlansPort {
  createPlan(input: CreateReadingPlanInput): Promise<ReadingPlan>;
  listPlans(): Promise<ReadingPlan[]>;
  setPlanStatus(id: string, status: ReadingPlanStatus): Promise<ReadingPlan>;
  generateToday(id: string): Promise<GenerateReadingDeliveryResult>;
  createSession(input: CreateReadingSessionInput): Promise<ReadingSessionResult>;
}
