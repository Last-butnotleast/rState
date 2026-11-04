import { createStore } from "@reference/store";
import { reset } from "@reference/reset";
import { undoRedo } from "@reference/undo";
import { validateSchemas } from "@reference/validate";
import { persist } from "@reference/persist";
import type { Context, Recorder } from "./contract";
import type { ValidationConfiguration, ValidationFactory } from "./validation";
import { referenceConfig } from "./reference-config";
import { wrapReference } from "./reference-wrapper";

type Issue = { message: string; path?: (string | number)[] };
type Schema = {
  "~standard": {
    version: 1;
    vendor: string;
    validate(value: any): { value?: unknown; issues?: Issue[] };
  };
};

function schema(
  target: string,
  name: string | undefined,
  record: Recorder,
  check: (value: any) => Issue[],
  configuration: ValidationConfiguration,
): Schema {
  return {
    "~standard": {
      version: 1,
      vendor: "rstate-fixture",
      validate: (value) => {
        record({
          kind: "validate",
          target,
          ...(name ? { name } : {}),
          value: structuredClone(value),
        });
        const issues = check(value);
        return issues.length
          ? { issues }
          : {
              value: configuration.alteredOutput
                ? { deliberatelyDifferent: true }
                : value,
            };
      },
    },
  };
}

function contextIssues(value: Context): Issue[] {
  if (value.count < 0 || value.count > 10)
    return [{ message: "count must be between 0 and 10", path: ["count"] }];
  const invalid = value.items.findIndex((price) => price < 0);
  if (invalid >= 0)
    return [
      {
        message: "price must be nonnegative",
        path: ["items", invalid],
      },
    ];
  if (value.total !== value.items.reduce((sum, price) => sum + price, 0))
    return [{ message: "total must equal item sum", path: ["total"] }];
  return [];
}

function schemas(configuration: ValidationConfiguration, record: Recorder) {
  const number =
    (field: "by" | "price") =>
    (value: any): Issue[] =>
      value[field] < 0 || value[field] > 5
        ? [
            {
              message: field + " must be between 0 and 5",
              path: [field],
            },
          ]
        : [];
  return {
    context: schema("context", undefined, record, contextIssues, configuration),
    events: {
      inc: schema("event", "inc", record, number("by"), configuration),
      bump: schema("event", "bump", record, number("by"), configuration),
      add: schema("event", "add", record, number("price"), configuration),
      recalculate: schema(
        "event",
        "recalculate",
        record,
        () => [{ message: "event is internal only" }],
        configuration,
      ),
      receipt: schema(
        "event",
        "receipt",
        record,
        () => [{ message: "event is internal only" }],
        configuration,
      ),
      ...Object.fromEntries(
        ["keep", "same", "equal", "fail", "start", "effectFail"].map((name) => [
          name,
          schema("event", name, record, () => [], configuration),
        ]),
      ),
    },
    emitted: configuration.missingReceipt
      ? {
          other: schema("emitted", "other", record, () => [], configuration),
        }
      : {
          receipt: schema(
            "emitted",
            "receipt",
            record,
            (value) =>
              value.total > 3
                ? [
                    {
                      message: "receipt total must not exceed 3",
                      path: ["total"],
                    },
                  ]
                : [],
            configuration,
          ),
        },
  };
}

export const createReferenceValidation: ValidationFactory = (
  configuration,
  record,
  storage,
) => {
  const config = referenceConfig("composition", record);
  config.on.start = (context, _event, enqueue) => {
    enqueue.effect((scope) =>
      record({ kind: "effect", count: scope.getSnapshot().context.count }),
    );
    enqueue.emit.receipt({ total: context.total });
    enqueue.trigger.inc({ by: 1 });
  };
  config.context.count = configuration.initialCount ?? 0;
  let resetCount: number | undefined;
  let store = createStore({
    ...config,
    ...(configuration.noSchemas
      ? {}
      : { schemas: schemas(configuration, record) }),
  }).with(
    reset({
      to: (initial) =>
        resetCount === undefined ? initial : { ...initial, count: resetCount },
    }),
  );
  if (configuration.historyLimit !== undefined)
    store = store.with(
      undoRedo({
        strategy: "snapshot",
        historyLimit: configuration.historyLimit,
      }),
    );
  if (configuration.persistence) {
    if (!storage) throw new Error("Storage adapter required");
    store = store.with(
      persist({
        ...configuration.persistence,
        strategy: "snapshot",
        storage,
      }),
    );
  }
  if (configuration.runtime ?? true)
    store = store.with(
      validateSchemas({
        context: configuration.context,
        events: configuration.events,
        emitted: configuration.emitted,
        unknownEvents: configuration.unknownEvents,
        unknownEmitted: configuration.unknownEmitted,
      }),
    );
  const base = wrapReference(store, configuration.historyLimit);
  return {
    ...base,
    metadata: () => ({
      context: Boolean(store.schemas?.context),
      events: Boolean(store.schemas?.events),
      emitted: Boolean(store.schemas?.emitted),
    }),
    history: () => ({
      past: store.getSnapshot().past?.length ?? 0,
      future: store.getSnapshot().future?.length ?? 0,
      limit: configuration.historyLimit ?? null,
    }),
    reset: () => store.send({ type: "reset" }),
    resetWithCount: (count) => {
      resetCount = count;
      try {
        store.send({ type: "reset" });
      } finally {
        resetCount = undefined;
      }
    },
    undo: () => store.send({ type: "undo" }),
    redo: () => store.send({ type: "redo" }),
  };
};
