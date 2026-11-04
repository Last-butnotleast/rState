# Core conformance scenarios

The browser tests run these declarative scenarios against the native Rust core compiled to WebAssembly and the pinned `@xstate/store` reference.

Reference: `@xstate/store` 4.3.0 at commit `9b3a88fa43f4be5433797acfba18f27c40494bd2`.

Each scenario has a descriptive `name`, a fixture `scenario`, and ordered `steps`. Fixture names are `counter`, `composition`, and `ordering`. Domain reducers are defined independently in the Rust adapter and JavaScript reference adapter; fixtures contain data rather than executable code.

| Operation | Fields | Purpose |
| --- | --- | --- |
| `subscribe` | `listener` | Register a named state observer. |
| `unsubscribe` | `listener` | Stop the named observer. |
| `send` | `event` | Dispatch an event synchronously. |
| `can` | `event` | Evaluate availability without publishing state or running effects. |
| `transition` | `event` | Evaluate a pure transition without changing the live store. |
| `snapshot` | None | Read the current state. |

`counter` covers values, keep versus explicit same-context replacement, equal-valued replacement identity, availability, pure transitions, and explicit unsubscription. `composition` covers internally triggered cart updates and final emissions. `ordering` covers nested effect dispatch and captured versus live effect snapshots.

Parity comparisons retain callback and command ordering, values, identity relationships, emitted events, effects, and structured recoverable errors. Identity is tracked separately from serialized values; equal contents do not imply equal snapshot or context identity.

Native tests under `crates/rstate/tests` cover the same public contracts plus deeper callback-error, cross-store notification, selection, and retained-effect behavior.

`factory.json` uses a separate, intentionally small multi-instance shape. Its `instances` entries name stores and their initialization inputs; every `step` includes a `store` name. The runner creates one shared factory definition, then creates the named stores through that definition. This avoids treating independently configured stores as evidence of factory behavior.

The factory runner supports the same operations above, plus `subscribeSelected` with `field` and `listener`, and `initial` to read the unchanged initial snapshot. Its two stores cover different inputs, isolated direct and selected subscriptions, pure transitions, availability, per-store emissions, initializer snapshot retention, reducer failure, and explicit unsubscription. All observations and emissions carry their store name.

`selection.json` describes a single explicit dependency chain: a source count object, a mapped `floor(count / 2)` bucket object, and a chained bucket label object. Its `read`, `subscribe`, `unsubscribe`, `send`, and `stats` operations compare retained value identities, projection counts, equality suppression, shared observation, final unsubscribe, lazy reads, and silent reactivation. Targets are `source`, `mapped`, and `chain`; `send.by` increments the count and zero requests an equal-valued context replacement. Inputs stay nonnegative so the Rust and JavaScript numeric projections agree. The reference uses real upstream computed atoms reading the source selection, while Rust uses the public `Selection::map` API.

`inspection.json` extends the ordinary runner with `inspect` and `subscribeSelected`. An inspector reports its current snapshot immediately, then the event and captured final snapshot for each received outer transition. The recorder also samples the store's current snapshot so nested dispatch can distinguish that state from an older outer record. The fixture covers unchanged events, silent `can` and pure transitions, late registration, selected/direct notification order, inspector-triggered dispatch, effect-triggered dispatch, captured effect reads, effect failure, and explicit cleanup. Separate adapter/native tests cover internal-trigger macrosteps, fail-fast inspector errors, failed initial registration, and live observer insertion/removal.

`combine.json` runs the same 32 operations in three fixed modes: `independent` creates two stores, `diamond` maps two branches from one source, and `duplicate` supplies the same selection twice. Rust calls `Selection::combine`; the oracle uses an actual computed atom that reads both input selections in declaration order. The result is `floor((left + right) / 2)`, followed by a label mapping. The modes compare lazy calculations, result identities, one coherent diamond notification, duplicate-input behavior, direct and derived listener ordering, shared observation, detachment, and silent reactivation. `send.input` names the input store; the diamond and duplicate modes share their underlying store. Independent sends are separate transitions. Values stay small and nonnegative so Rust and JavaScript numeric projections are equivalent.

`controls.json` uses one input-configured store with a bounded snapshot history. Its runner adds `reset`, `resetWithCount`, `undo`, `redo`, `initial` and `history` operations while preserving state/selected callbacks, actual control inspection, context/snapshot identity and effects. The reference composes the pinned `reset` and snapshot `undoRedo` extensions in that order. Ordinary no-op events consume history; internal trigger chains consume one entry; snapshot undo/redo never rerun effects. Empty controls, capacity, reset restoration, redo branching and failure after commit are covered.

`persistence.json` adds the pinned snapshot `persist` extension last and supplies the same synchronous storage contract to both adapters. The trace includes actual reads, writes and clears with parsed versioned context envelopes. It checks hydrated initial state, configured reset defaults, empty initial history, undo/redo/reset persistence and clear without changing live state. Separate browser tests cover post-inspection write ordering, stale outer revisions, nested clear epochs, automatic-save failure and actual localStorage reload.

Native policy differences are tested explicitly rather than normalized out of parity traces: domain `can` preserves its allowedness, persistence bookkeeping does not allocate metadata-only snapshots, empty controls do not publish, load corruption/version mismatch is an explicit error, and `save()` retries the current snapshot. Upstream `flushStorage` only flushes pending writes and is not substituted for native force-save. Storage error traces compare the original causal message; typed native error stages and version diagnostics are tested separately. No undo history is serialized.
