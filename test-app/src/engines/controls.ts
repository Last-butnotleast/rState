import type { Engine, Event, Recorder } from "./contract";

export type ControlConfiguration = {
  scenario?: "counter" | "composition" | "ordering";
  initialCount?: number;
  historyLimit?: number;
  persistence?: { name: string; version: number };
};

export type StorageAdapter = {
  getItem(name: string): string | null;
  setItem(name: string, value: string): void;
  removeItem(name: string): void;
};

export type HistoryStatus = {
  past: number;
  future: number;
  limit: number | null;
};

export interface ControlEngine extends Engine {
  reset(): void;
  resetWithCount(count: number): void;
  undo(): void;
  redo(): void;
  history(): HistoryStatus;
  save(): void;
  clearStorage(): void;
  persistenceError(): string | null;
}

export type ControlFactory = (
  configuration: ControlConfiguration,
  record: Recorder,
  storage?: StorageAdapter,
) => ControlEngine;

export type ControlOperation = {
  op:
    | "send"
    | "reset"
    | "resetWithCount"
    | "undo"
    | "redo"
    | "snapshot"
    | "initial"
    | "history"
    | "subscribe"
    | "subscribeSelected"
    | "inspect"
    | "unsubscribe"
    | "save"
    | "clearStorage"
    | "persistenceError";
  event?: Event;
  count?: number;
  listener?: string;
  field?: "count" | "total";
};

export type ControlScenario = {
  name: string;
  configuration: ControlConfiguration;
  stored?: {
    context: { count: number; items: number[]; total: number };
    version: number;
  };
  steps: ControlOperation[];
};
