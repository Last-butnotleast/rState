export type ScenarioName = "counter" | "composition" | "ordering";
export type Event = { type: string; by?: number; price?: number };
export type Context = { count: number; items: number[]; total: number };
export type Snapshot = {
  context: Context;
  snapshotId: number;
  contextId: number;
};
export type Message = { type: string; total?: number };
export type TraceEntry = Record<string, unknown>;
export type Recorder = (entry: TraceEntry) => void;
export type Inspection = {
  event: Event;
  snapshot: Snapshot;
};

export interface Engine {
  snapshot(): Snapshot;
  initialSnapshot(): Snapshot;
  send(event: Event): void;
  can(event: Event): boolean;
  transition(event: Event): {
    snapshot: Snapshot;
    effects: number;
    allowed: boolean;
  };
  subscribe(callback: (snapshot: Snapshot) => void): () => void;
  inspect(callback: (record: Inspection) => void): () => void;
  subscribeSelected(
    field: "count" | "total",
    callback: (value: number) => void,
  ): () => void;
  onReceipt(callback: (message: Message) => void): () => void;
  onAny(callback: (message: Message) => void): () => void;
  dispose(): void;
}

export type Operation = {
  op:
    | "subscribe"
    | "subscribeSelected"
    | "inspect"
    | "unsubscribe"
    | "send"
    | "can"
    | "transition"
    | "snapshot";
  field?: "count" | "total";
  listener?: string;
  event?: Event;
  send?: Event;
  whenCount?: number;
  fail?: string;
};

export type Scenario = {
  name: string;
  scenario: ScenarioName;
  steps: Operation[];
};
