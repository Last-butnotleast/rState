# Public API stability decisions

These decisions prepare the existing synchronous, single-threaded API for a future 1.0 release. They do not change the package version or promise compatibility with every `@xstate/store` feature. The dependency and language baseline remain those in `Cargo.toml`.

The policy follows the [Cargo SemVer compatibility guidance](https://doc.rust-lang.org/cargo/reference/semver.html) and the [Rust Reference for `non_exhaustive`](https://doc.rust-lang.org/reference/attributes/type_system.html#the-non_exhaustive-attribute). Adding a variant to an exhaustive public enum, adding a field to an externally constructible record, or adding `non_exhaustive` after release can break downstream code. Extensibility is selected by each type's purpose.

## Open diagnostics and records

| Public surface | Decision and caller contract |
| --- | --- |
| `StoreError` | Opaque struct. Construct callback errors with `new`, application errors with `from_error`, and validation errors through `From<ValidationError>`. Inspect with `dispatch_status`, `validation_error`, `Display` and `Error::source`. The internal cause representation can evolve. |
| `PersistenceError`, `PersistenceOperation`, `ValidationStage`, `ValidationReason`, `ControlAction` | Non-exhaustive enums. These library diagnostics and lifecycle descriptions can gain cases. Downstream matches need a fallback arm. |
| `PersistenceActivity`, `PersistenceOutcome` | Non-exhaustive observation enums. Outcomes describe the reporting persistence operation; they are not saved/dirty proofs. Include a fallback arm. |
| `PersistenceStatus` | Opaque shared observation with `sequence`, `last_activity`, `outcome` and `error` accessors. Its private representation can evolve without adding application trait bounds. |
| `PersistenceError::Operation`, `VersionMismatch`, `Reentrant` | Non-exhaustive named-field variants. Match needed fields with `..`; these library-created envelopes may gain diagnostic fields. They are not public construction protocols. |
| `Inspection` | Non-exhaustive enum with non-exhaustive `Initial`, `Transition` and `Control` variants. New records or metadata can be added. Match fields with `..` and include a fallback arm. |
| `ValidationOptions` | Non-exhaustive configuration record with readable and mutable public fields. Begin with `Default` and use `with_context`, `with_events`, `with_emitted`, `with_unknown_events` and `with_unknown_emitted`, or assign individual fields. Future switches receive compatible defaults. |
| `ValidationError` | Non-exhaustive diagnostic record. Public fields remain readable. Application-provided validation errors use `new(stage, reason, schema)`, optionally followed by `named` and `with_issues`. |
| `ValidationIssue`, `SchemaMetadata` | Non-exhaustive records with their existing constructors and builders. Use `ValidationIssue::new`/`with_path` and `SchemaMetadata::new`/`with_vendor`. |
| `HistoryStatus`, `TransitionResult`, `CommandResult` | Non-exhaustive output records. Read public fields or destructure with `..`. Stores and transition queries produce their values; future diagnostic fields do not require downstream struct-literal changes. `HistoryStatus::default` remains available. |
| `TransitionOutcome` | Non-exhaustive outcome enum. Match `Applied`, `Unavailable`, `Rejected` and `ValidationFailed`, with a fallback arm for future outcomes. Application rejection reasons remain generic. |
| Store, transition, selection, schema, persistence, subscription, factory and scope handles | Existing private fields already protect representation choices. Their documented behavior, trait implementations and generic bounds remain public contracts. |

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

## Versioned context restoration

`Persisted<C>` keeps its existing closed `{ context, version: u64 }` construction and serialization shape. Its optional Serde implementations require `serde`; the existing `json` feature includes that feature and continues to encode the same shape. Application context bounds remain local to serialization operations.

`Persisted::into_snapshot(expected_version)` consumes an envelope and returns `StoreResult<Rc<Snapshot<C>>>` without context trait bounds or store initialization. A mismatch retains the existing typed `PersistenceError::VersionMismatch` source and carries no dispatch status. Successful restoration creates a shared snapshot with no history and does not invoke schema callbacks. Application-defined typed migrations run before this version check; codecs may apply those same migrations before returning a target-version envelope to the unchanged store loader. No automatic migration registry or storage policy is introduced.

## Stateless transition boundary

`Transition<C, E, M>` is a cloneable reducer definition created without an initial context or store lifecycle. `transition(&Rc<Snapshot<C>>, E)` returns the existing `StoreResult<TransitionResult<C, E, M>>`. Its opaque representation and `Debug` implementation add no application trait bounds. Reducers and optional schema callbacks are shared across clones; the handle owns no current snapshot, subscribers, persistence or history configuration.

Stateless and store-backed queries share reduction and opt-in validation. They collect effects without executing them, preserve the existing event availability and snapshot identity rules, and clear nested dispatch status at their pure-query boundary. `with_schemas` alone is inert; `with_validation` checks the outer event and final context/emissions without validating initial external context. `Transition` does not extend snapshot history; existing `Store::transition` keeps its configured history behavior. Deterministic outputs require pure application reducers and validators and immutable context values.

This is a native API extension analogous to the reference's standalone transition constructor. It does not add workflow behavior, a runtime or a new command protocol, and it leaves existing `Store` signatures and behavior unchanged.

`TransitionResult::into_commands` returns `Result<CommandResult<C, M>, TransitionResult<C, E, M>>`. The success record contains the existing proposed snapshot, `Vec<Rc<M>>` commands and availability. It reuses emitted values without additional generic bounds. A proposal containing any executable callback is returned intact as the error value; conversion cannot silently omit executable effects. Payload serialization remains application-owned and optional.

`Transition::outcome` classifies the existing result into `Applied`, `Unavailable` or `ValidationFailed`. Validation outcomes retain the original `StoreError` and source; operational callback failures remain `Err`. `outcome_with_guard` also accepts a per-call `FnOnce(&Snapshot<C>, &E) -> Result<(), R>` and reports `Rejected { reason: R }` before validation or reduction. The guard runs once for the outer event, adds no trait bounds to its reason and does not change internal-trigger behavior. Rejected and invalid outcomes expose no effects. Input snapshots remain externally owned, with the same purity and interior-mutation qualifications as ordinary transitions. These structured queries leave existing Store APIs, availability, effects and validation behavior unchanged.

## Replay boundary

`replay` and `replay_with_guard` take an initial `Rc<Snapshot<C>>`, an `IntoIterator<Item = E>` and an explicit `ReplayPolicy`. The guard variant adds `FnMut(&Snapshot<C>, &E) -> Result<(), R>` and delegates each outer event to the existing guarded outcome API. The utility introduces no application `Clone`, serialization, thread-safety or static-lifetime bounds. Conditional `Debug` on `ReplayResult` follows the output types' existing context and rejection-reason requirements without adding event or command bounds.

`ReplayPolicy` is a closed enum with `Stop` and `Skip`; its default is `Stop`. Both policies retain non-applied outcomes. `Stop` includes the stopping outcome and does not request another iterator item; `Skip` continues from the last applied snapshot. A non-validation callback error always returns `Err(StoreError)` with its original source and no dispatch status, discarding partial proposals. `ReplayResult` is a non-exhaustive output record with readable `snapshot` and ordered `outcomes` fields. Empty replay preserves the initial snapshot identity.

Replay retains applied effects without executing them and adds no publication, persistence, runtime or history behavior. It shares ordinary transitions' purity, snapshot identity, internal-trigger and validation contracts. The caller owns event storage and durable command interpretation.

## Snapshot selection boundary

`Store::select_snapshot` projects `&Snapshot<C>` and returns the existing `Selection<T>` handle. It shares context selections' lazy reads, snapshot identity cache, comparator-controlled value identity, explicit composition and cleanup semantics without adding context or selected-value trait bounds. Snapshot metadata is a native extension; it does not imply additional `@xstate/store` compatibility. History-only snapshots can change a snapshot projection while an equal context projection retains its value identity. Persistence bookkeeping remains outside this observation boundary.

## Persistence observations remain outside snapshots

`Store::persistence_status` returns `Option<Rc<PersistenceStatus>>`; unconfigured stores return `None`. `subscribe_persistence` takes an infallible callback and returns the ordinary `StoreResult<Subscription>`, reporting `NotConfigured` when needed. There is no registration replay; dropping an ordinary handle leaves observation active, while scoped cleanup unsubscribes. Status reads retain shared identity until another outcome is recorded.

Status is a latest observation with a monotonic sequence, not an operation log. Automatic encoding/storage failures and successful writes record outcomes; skipped automatic requests do not imply persistence succeeded. Explicit save and clear record their returned outcome after their existing aggregation rules. A clear may succeed even when a nested automatic write failed, so its successful outcome can coexist with a last storage error. `error()` captures the existing `persistence_error()` state when the observation is recorded; rejected or superseded explicit operations do not manufacture a storage error. An in-flight explicit operation does not record its completed status yet, so its legacy error callback can read a previous status alongside a newer live `persistence_error()`. After recording, the status retains its own immutable error handle. No field promises the current domain revision is saved or clean.

Nested status updates can coalesce and sequence numbers can jump. Callbacks run only after the outer persistence busy and active-I/O scopes end; a callback may retry, clear or send synchronously. A listener already running is not reentered, and subsequent listeners read the latest status. Automatic attempts record their observation before invoking the legacy error callback so nested clear/recovery outcomes are retained. Explicit outcomes reflect completion of the reporting call before feedback delivery; newer operations performed by feedback can replace that observation. Status notification uses a separate infallible registry and cannot drain or suppress pending fallible domain notifications. Captures are released outside interior borrows. At the final operation boundary, retired observation and discarded error captures are released after the busy guard, so destructor-triggered nested commits can finish their own persistence work.

Persistence status is a native extension with no additional upstream parity claim. It preserves domain snapshot identity, storage ordering, existing error channels and the core's synchronous single-threaded bounds. Callback panics remain programming faults.

## Dispatch errors report the owning call

`send`, `reset`, `reset_with`, `undo` and `redo` retain their `StoreResult<()>` signatures. Their returned errors carry `Some(DispatchStatus::NotCommitted)` before that operation publishes its proposed state/history, or `Some(DispatchStatus::Committed)` after that boundary. `EffectScope::send` forwards the nested dispatch result unchanged. Snapshot identity is not used to guess the outcome: no-op events and empty undo/redo still cross the delivery boundary.

Reducer, schema or reset-provider errors before commit leave that operation's proposed state and history unpublished. After commit, subscribers run before inspection, persistence and user effects. Subscriber failures drain scheduled notifications and skip the reporting dispatch's later stages. Inspector, emission and effect failures stop their remaining pipeline. Committed state and history remain committed. Automatic persistence failures keep their separate reporting contract and do not become dispatch failures.

Nested callbacks can dispatch synchronously. A callback can propagate a nested error, but the outer operation assigns its own status when returning it. An outer evaluation can return `NotCommitted` after a nested event committed; an outer post-commit failure is `Committed` even if its cause was a nested rejection. Status is not evidence that the live snapshot still belongs to that operation, that no application side effect occurred, or that retrying is safe. Panics remain programming faults outside this recoverable result contract.

`StoreError::new`, `from_error` and conversion from `ValidationError` initially carry no dispatch status. Pure `transition`/`can`, direct schema checks, factory initialization, inspection registration and explicit persistence operations report their own non-dispatch error boundary and do not inherit a callback's nested dispatch status. `Error::source()` exposes the original concrete application or validation error directly; recording status does not insert a wrapper into the causal chain or change the displayed message.

## Initial inspection failure cleans up

Inspection registration happens before the immediate `Initial` callback, preserving synchronous nested dispatch and live listener ordering. If that callback returns `Err`, the new registration is removed before `inspect` returns. Cleanup releases callback captures outside interior borrows. Other registrations, events dispatched during the callback and their committed state are retained.

This intentionally differs from the pinned JavaScript reference, which retains the failed registration without returning a handle. Successful registrations still require explicit unsubscribe or an opt-in scoped guard; dropping an ordinary subscription is unchanged. An initial-registration error has no dispatch status even if its callback propagated a nested dispatch failure. A panic is not converted into a callback error or a rollback guarantee.

## GPUI command ownership

The separate adapter exposes a non-cloneable `CommandScope` owner, cloneable `CommandChannel` submission handles and request tickets. Channels and tickets do not extend scope ownership. Scope closure rejects later submissions and disables delivery; dropping the owner closes it. Dropping a ticket leaves its command owned by the scope, while explicit cancellation and invalidation suppress delivery. Request identities remain outside store snapshots and history.

`Concurrent`, `Serial` and `Latest` are explicit channel policies. Submission schedules foreground startup after synchronous callers return. Serial channels retain already-started canceled work until completion before starting their next request. Cancellation does not roll back external side effects or guarantee interruption of blocking operating-system work. Ordering is local to the chosen channel and scope.

Work outputs remain application-typed, including expected operation errors. Completion callbacks return the core's `StoreResult`; a separate diagnostics callback receives delivery errors unchanged, without retry or rollback. Foreground work and delivery can use single-threaded state, while background work and its result must satisfy GPUI's thread-safety bounds. The supported GPUI package identity, desktop toolchain and platform requirements belong to the adapter compatibility contract, independently of the core's synchronous API and Rust baseline.
