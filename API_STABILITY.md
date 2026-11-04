# Public API stability decisions

These decisions prepare the existing synchronous, single-threaded API for a future 1.0 release. They do not change the package version or promise compatibility with every `@xstate/store` feature. The dependency and language baseline remain those in `Cargo.toml`.

The policy follows the [Cargo SemVer compatibility guidance](https://doc.rust-lang.org/cargo/reference/semver.html) and the [Rust Reference for `non_exhaustive`](https://doc.rust-lang.org/reference/attributes/type_system.html#the-non_exhaustive-attribute). Adding a variant to an exhaustive public enum, adding a field to an externally constructible record, or adding `non_exhaustive` after release can break downstream code. Extensibility is selected by each type's purpose.

## Open diagnostics and records

| Public surface | Decision and caller contract |
| --- | --- |
| `StoreError` | Opaque struct. Construct callback errors with `new`, application errors with `from_error`, and validation errors through `From<ValidationError>`. Inspect with `dispatch_status`, `validation_error`, `Display` and `Error::source`. The internal cause representation can evolve. |
| `PersistenceError`, `PersistenceOperation`, `ValidationStage`, `ValidationReason`, `ControlAction` | Non-exhaustive enums. These library diagnostics and lifecycle descriptions can gain cases. Downstream matches need a fallback arm. |
| `PersistenceError::Operation`, `VersionMismatch`, `Reentrant` | Non-exhaustive named-field variants. Match needed fields with `..`; these library-created envelopes may gain diagnostic fields. They are not public construction protocols. |
| `Inspection` | Non-exhaustive enum with non-exhaustive `Initial`, `Transition` and `Control` variants. New records or metadata can be added. Match fields with `..` and include a fallback arm. |
| `ValidationOptions` | Non-exhaustive configuration record with readable and mutable public fields. Begin with `Default` and use `with_context`, `with_events`, `with_emitted`, `with_unknown_events` and `with_unknown_emitted`, or assign individual fields. Future switches receive compatible defaults. |
| `ValidationError` | Non-exhaustive diagnostic record. Public fields remain readable. Application-provided validation errors use `new(stage, reason, schema)`, optionally followed by `named` and `with_issues`. |
| `ValidationIssue`, `SchemaMetadata` | Non-exhaustive records with their existing constructors and builders. Use `ValidationIssue::new`/`with_path` and `SchemaMetadata::new`/`with_vendor`. |
| `HistoryStatus`, `TransitionResult` | Non-exhaustive output records. Read public fields or destructure with `..`. Stores and transition queries produce their values; future diagnostic fields do not require downstream struct-literal changes. `HistoryStatus::default` remains available. |
| Store, selection, schema, persistence, subscription, factory and scope handles | Existing private fields already protect representation choices. Their documented behavior, trait implementations and generic bounds remain public contracts. |

For example, validation options no longer use external struct literals or struct-update syntax:

```rust
use rstate::{UnknownPolicy, ValidationOptions};

let options = ValidationOptions::default()
    .with_context(true)
    .with_events(true)
    .with_emitted(false)
    .with_unknown_events(UnknownPolicy::Ignore);
```

This is a deliberate pre-1.0 source change. Replace `StoreError` variant construction and matching with constructors and accessors, replace growable record literals with their supported creation methods, and add fallback patterns when consuming open enums. No `Clone`, serialization, `Send` or `Sync` bounds are added to contexts, events, emissions or selected values.

## Deliberately closed protocols

| Public surface | Stable set of cases |
| --- | --- |
| `DispatchStatus` | `NotCommitted` and `Committed`: whether the reporting call crossed its own commit boundary. |
| `Update` | Keep the context or replace its shared handle. |
| `Effect` | Emit a value or run a synchronous callback. |
| `SchemaCheck` | Valid, invalid or unknown. |
| `UnknownPolicy` | Reject or ignore. |
| `PathSegment` | A field name or an index. |
| `Persisted<Context>` | The context and its numeric version, used as the application codec's construction contract. |

These types represent complete application-facing choices or a concrete interchange shape. Adding cases or required fields would require downstream decisions, so such changes deserve a major-version review rather than a catch-all pattern today.

`SnapshotStorage` and `SnapshotCodec` remain open traits intended for application implementations. New required methods or changed signatures require a major-version review; compatible new behavior should use defaults or separate optional interfaces.

## Snapshot selection boundary

`Store::select_snapshot` projects `&Snapshot<C>` and returns the existing `Selection<T>` handle. It shares context selections' lazy reads, snapshot identity cache, comparator-controlled value identity, explicit composition and cleanup semantics without adding context or selected-value trait bounds. Snapshot metadata is a native extension; it does not imply additional `@xstate/store` compatibility. History-only snapshots can change a snapshot projection while an equal context projection retains its value identity. Persistence bookkeeping remains outside this observation boundary.

## Dispatch errors report the owning call

`send`, `reset`, `reset_with`, `undo` and `redo` retain their `StoreResult<()>` signatures. Their returned errors carry `Some(DispatchStatus::NotCommitted)` before that operation publishes its proposed state/history, or `Some(DispatchStatus::Committed)` after that boundary. `EffectScope::send` forwards the nested dispatch result unchanged. Snapshot identity is not used to guess the outcome: no-op events and empty undo/redo still cross the delivery boundary.

Reducer, schema or reset-provider errors before commit leave that operation's proposed state and history unpublished. After commit, subscribers run before inspection, persistence and user effects. Subscriber failures drain scheduled notifications and skip the reporting dispatch's later stages. Inspector, emission and effect failures stop their remaining pipeline. Committed state and history remain committed. Automatic persistence failures keep their separate reporting contract and do not become dispatch failures.

Nested callbacks can dispatch synchronously. A callback can propagate a nested error, but the outer operation assigns its own status when returning it. An outer evaluation can return `NotCommitted` after a nested event committed; an outer post-commit failure is `Committed` even if its cause was a nested rejection. Status is not evidence that the live snapshot still belongs to that operation, that no application side effect occurred, or that retrying is safe. Panics remain programming faults outside this recoverable result contract.

`StoreError::new`, `from_error` and conversion from `ValidationError` initially carry no dispatch status. Pure `transition`/`can`, direct schema checks, factory initialization, inspection registration and explicit persistence operations report their own non-dispatch error boundary and do not inherit a callback's nested dispatch status. `Error::source()` exposes the original concrete application or validation error directly; recording status does not insert a wrapper into the causal chain or change the displayed message.

## Initial inspection failure cleans up

Inspection registration happens before the immediate `Initial` callback, preserving synchronous nested dispatch and live listener ordering. If that callback returns `Err`, the new registration is removed before `inspect` returns. Cleanup releases callback captures outside interior borrows. Other registrations, events dispatched during the callback and their committed state are retained.

This intentionally differs from the pinned JavaScript reference, which retains the failed registration without returning a handle. Successful registrations still require explicit unsubscribe or an opt-in scoped guard; dropping an ordinary subscription is unchanged. An initial-registration error has no dispatch status even if its callback propagated a nested dispatch failure. A panic is not converted into a callback error or a rollback guarantee.
