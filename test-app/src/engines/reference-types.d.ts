declare module "@reference/store" {
  type Context = { count: number; items: number[]; total: number };
  type Event = { type: string; by?: number; price?: number };
  type Message = { type: string; total?: number };

  type SourceSnapshot = { context: Context };
  type Subscription = { unsubscribe(): void };
  type Scope = { send(event: Event): void; getSnapshot(): SourceSnapshot };
  type Queue = {
    effect(callback: (scope: Scope) => void): void;
    trigger: Record<string, (payload?: Record<string, unknown>) => void>;
    emit: Record<string, (payload?: Record<string, unknown>) => void>;
  };
  export type Handler = (
    context: Context,
    event: Event,
    enqueue: Queue,
  ) => Context | void;
  type Result = [SourceSnapshot, unknown[]] & { _allowed?: boolean };
  export type SourceStore = {
    send(event: Event): void;
    getSnapshot(): SourceSnapshot;
    getInitialSnapshot(): SourceSnapshot;
    transition(snapshot: SourceSnapshot, event: Event): Result;
    subscribe(callback: (snapshot: SourceSnapshot) => void): Subscription;
    inspect(
      callback: (record: { event: Event; snapshot: SourceSnapshot }) => void,
    ): Subscription;
    select<T>(
      selector: (context: Context) => T,
      compare: (left: T, right: T) => boolean,
    ): import("@reference/atom").ReadonlyAtom<T>;
    on(type: string, callback: (message: Message) => void): Subscription;
    can: Record<string, (payload?: Record<string, unknown>) => boolean>;
  };
  export function createStore(config: {
    context: Context;
    on: Record<string, Handler>;
  }): SourceStore;
  export function createStoreLogic(config: {
    context: (input: { count: number }) => Context;
    on: Record<string, Handler>;
  }): { createStore(input: { count: number }): SourceStore };
}

declare module "@reference/atom" {
  export type ReadonlyAtom<T> = {
    get(): T;
    subscribe(callback: (value: T) => void): { unsubscribe(): void };
  };
  export function createAtom<T>(
    getter: () => T,
    options?: { compare(left: T, right: T): boolean },
  ): ReadonlyAtom<T>;
}
