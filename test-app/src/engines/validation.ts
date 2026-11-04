import type { Engine, Event, Recorder, TraceEntry } from "./contract";
import type { StorageAdapter } from "./controls";

export type ValidationConfiguration = {
  runtime?: boolean;
  initialCount?: number;
  historyLimit?: number;
  noSchemas?: boolean;
  context?: boolean;
  events?: boolean;
  emitted?: boolean;
  unknownEvents?: "throw" | "ignore";
  unknownEmitted?: "throw" | "ignore";
  missingReceipt?: boolean;
  alteredOutput?: boolean;
  persistence?: { name: string; version: number };
};

export type ValidationMetadata = {
  context: boolean;
  events: boolean;
  emitted: boolean;
};

export interface ValidationEngine extends Engine {
  metadata(): ValidationMetadata;
  history(): { past: number; future: number; limit: number | null };
  reset(): void;
  resetWithCount(count: number): void;
  undo(): void;
  redo(): void;
}

export type ValidationFactory = (
  configuration: ValidationConfiguration,
  record: Recorder,
  storage?: StorageAdapter,
) => ValidationEngine;

export type ValidationOperation = {
  op:
    | "send"
    | "can"
    | "transition"
    | "snapshot"
    | "history"
    | "metadata"
    | "subscribe"
    | "inspect"
    | "reset"
    | "resetWithCount"
    | "undo"
    | "redo";
  event?: Event;
  count?: number;
};

export type ValidationScenario = {
  name: string;
  configuration: ValidationConfiguration;
  steps: ValidationOperation[];
};

export function validationFailure(failure: unknown): TraceEntry {
  const error = failure as {
    reason?: string;
    eventType?: string;
    issues?: { message: string; path?: (string | number)[] }[];
    message?: string;
  };
  return error?.reason
    ? {
        reason: error.reason,
        eventType: error.eventType ?? null,
        issues: (error.issues ?? []).map((issue) => ({
          message: issue.message,
          path: issue.path ?? [],
        })),
      }
    : {
        message: failure instanceof Error ? failure.message : String(failure),
      };
}
