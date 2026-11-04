import { clearStorage, persist } from "@reference/persist";
import { reset } from "@reference/reset";
import { createStore } from "@reference/store";
import { undoRedo } from "@reference/undo";
import type { ControlFactory } from "./controls";
import { referenceConfig } from "./reference-config";
import { wrapReference } from "./reference-wrapper";

export const createReferenceControls: ControlFactory = (
  configuration,
  record,
  storage,
) => {
  let resetCount: number | undefined;
  let lastError: string | null = null;
  const config = referenceConfig(configuration.scenario ?? "counter", record);
  config.context = {
    count: configuration.initialCount ?? 0,
    items: [],
    total: 0,
  };
  let store = createStore(config).with(
    reset({
      to: (initial) =>
        resetCount === undefined ? initial : { ...initial, count: resetCount },
    }),
  );
  if (configuration.historyLimit !== undefined) {
    store = store.with(
      undoRedo({
        strategy: "snapshot",
        historyLimit: configuration.historyLimit,
      }),
    );
  }
  if (configuration.persistence) {
    if (!storage) throw new Error("Storage adapter required");
    store = store.with(
      persist({
        ...configuration.persistence,
        strategy: "snapshot",
        storage,
        onError: (failure) => {
          lastError =
            failure instanceof Error ? failure.message : String(failure);
          record({ kind: "persistError", message: lastError });
        },
        onDone: () => {
          lastError = null;
        },
      }),
    );
  }
  const engine = wrapReference(store, configuration.historyLimit);
  return {
    ...engine,
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
    history: () => ({
      past: store.getSnapshot().past?.length ?? 0,
      future: store.getSnapshot().future?.length ?? 0,
      limit: configuration.historyLimit ?? null,
    }),
    save: () => {
      throw new Error("Explicit current snapshot save is native-only");
    },
    clearStorage: () => clearStorage(store),
    persistenceError: () => lastError,
  };
};
