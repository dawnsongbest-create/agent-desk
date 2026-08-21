import type { IngestDeliveryResult } from "./delivery";
import type { ReaderDocument } from "./reader";

export const readingDifficulties = ["normal", "technical"] as const;
export type ReadingDifficulty = (typeof readingDifficulties)[number];

export const readingPlanStatuses = ["active", "paused", "completed"] as const;
export type ReadingPlanStatus = (typeof readingPlanStatuses)[number];

export type CreateReadingPlanInput = {
  title: string;
  sourceName: string | null;
  contentMarkdown: string;
  dailyMinutes: number;
  scheduleTime: string;
  difficulty: ReadingDifficulty;
};

export type ReadingPlan = {
  id: string;
  title: string;
  sourceName: string | null;
  contentMarkdown: string;
  totalContentLength: number;
  dailyMinutes: number;
  scheduleTime: string;
  difficulty: ReadingDifficulty;
  status: ReadingPlanStatus;
  currentOffset: number;
  currentDay: number;
  createdAt: string;
  updatedAt: string;
};

export type ReadingPlanDelivery = {
  planId: string;
  day: number;
  deliveryId: string;
  documentId: string;
  contentStart: number;
  contentEnd: number;
  estimatedMinutes: number;
  createdAt: string;
};

export type GenerateReadingDeliveryResult = {
  plan: ReadingPlan;
  delivery: IngestDeliveryResult;
  generation: ReadingPlanDelivery;
};

export type ReadingSession = {
  id: string;
  sourceDocumentId: string;
  readerDocumentId: string;
  content: string;
  estimatedMinutes: number;
  createdAt: string;
};

export type ReadingSessionResult = {
  session: ReadingSession;
  document: ReaderDocument;
};

export type CreateReadingSessionInput = {
  sourceDocumentId: string;
  content: string;
};
