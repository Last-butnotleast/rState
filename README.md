# rstate

A small synchronous Rust state store with typed events, shared snapshots, pure transitions, subscriptions, effects, emitted events, transition inspection, context selections, reusable factories, reset, snapshot history, synchronous snapshot persistence and opt-in validation. The default library uses the Rust standard library and `thiserror`; the optional `json` feature adds a serialization adapter.

The selected behavior follows [`@xstate/store` 4.3.0 at commit `9b3a88f`](https://github.com/statelyai/xstate/tree/9b3a88fa43f4be5433797acfba18f27c40494bd2/packages/xstate-store). This is a core implementation, not a port of the complete package or its extensions.

[API_STABILITY.md](API_STABILITY.md) records the pre-1.0 public API decisions, extensible types, dispatch failure contract and intentional inspection-cleanup difference from the reference.

## Using the store

```rust
use rstate::{Store, StoreConfig, Update};

struct Counter {
    count: i32,
}

enum Event {
    Increment(i32),
    Keep,
}

let store: Store<Counter, Event> = Store::new(StoreConfig::new(
    Counter { count: 0 },
    |context, event, _| {
        Ok(match event {
            Event::Increment(by) => Update::replace(Counter {
                count: context.count + by,
            }),
            Event::Keep => Update::Keep,
        })
    },
));

let subscription = store.subscribe(|snapshot| {
    println!("{}", snapshot.context.count);
    Ok(())
});

store.send(Event::Increment(1)).unwrap();
assert_eq!(store.get_snapshot().context.count, 1);
assert!(!store.can(Event::Keep).unwrap());
subscription.unsubscribe();
```

Reducers return `StoreResult<Update<Context>>`. Use `StoreError::new` for a callback failure or `StoreError::from_error` to retain an application error as its direct source, including its existing source chain. External errors must implement `Error + Send + Sync + 'static`, so `StoreError` can propagate into application error containers such as `Box<dyn Error + Send + Sync>` or `anyhow::Error`. These bounds apply to external errors, not to store contexts, events or callbacks. Callback failures are recoverable results; Rust panics are programming faults rather than JavaScript-compatible exceptions.

`StoreError` has an opaque representation. Read validation details through `validation_error()`, application causes through `Error::source()`, and dispatch outcome through `dispatch_status()`. `send`, `reset`, `reset_with`, `undo` and `redo` return errors marked `Some(DispatchStatus::NotCommitted)` when that call fails before its commit boundary, or `Some(DispatchStatus::Committed)` when delivery fails after committing. The method signatures remain `StoreResult<()>`; an error does not imply rollback.

```rust
use rstate::DispatchStatus;

if let Err(error) = store.send(Event::Increment(1)) {
    match error.dispatch_status() {
        Some(DispatchStatus::Committed) => {
            eprintln!("state committed, delivery failed: {error}");
        }
        Some(DispatchStatus::NotCommitted) => {
            eprintln!("this event did not commit: {error}");
        }
        None => eprintln!("operation failed: {error}"),
    }
}
```

The status describes the reporting call, including an unchanged-context or empty-history commit. It does not describe the latest live snapshot or whether callbacks dispatched other events: nested operations can commit even when the outer call is `NotCommitted`, and later nested state can supersede a `Committed` snapshot. Do not retry an event automatically based on status alone. Pure `transition` and `can` queries, schema checks, initialization, inspection registration and explicit persistence errors have no dispatch status; propagated nested dispatch metadata is cleared at those operation boundaries.

`Update::Keep` retains context and snapshot identity. `Update::Replace` replaces the complete context; equal contents do not imply equal identity. Returning the existing context handle preserves snapshot identity while making `can` true. Contexts, events and selected values do not need `Clone`, `Serialize`, `Send` or `Sync`.

`store.transition(snapshot, event)` returns a proposed snapshot, ordered effects and availability without publishing or running effects. `store.can(event)` evaluates the same reducer and discards its proposed changes and effects. Keep application side effects inside `enqueue.effect`, because reducer code itself runs during these queries.

`enqueue.trigger(event)` processes internal events FIFO within one transition and publishes only its final snapshot. `enqueue.emit(message)` and `enqueue.effect(callback)` retain their collection order. Callback calls to `store.send` reenter synchronously. Shared notification scheduling reconciles nested updates across stores before nested sends return.

Effects receive a cloneable `EffectScope` with `send` and `get_snapshot`. Snapshot reads use that dispatch's snapshot during its entire effect loop, then read the current store after successful completion. If an effect fails, retained scopes remain pinned to that dispatch's snapshot, matching the reference.

Subscriptions do not replay their initial value. Dropping a subscription handle leaves registration active; call `unsubscribe` explicitly. It is idempotent. A subscriber failure preserves committed state, drains remaining scheduled notifications, returns the first failure and skips that dispatch's effects. Emitted listeners and effects fail immediately, retaining committed state. Filtered emitted listeners run before wildcard listeners.

Use `subscription.scoped()` to transfer a registration into a non-cloneable `ScopedSubscription` guard. Dropping the guard unsubscribes automatically; `guard.unsubscribe()` ends it earlier and remains idempotent. An ordinary cloned subscription refers to the same registration, so dropping a scoped guard also deactivates its ordinary aliases. Keep the guard alive for the desired observation lifetime. Store, selection, inspection and emitted-event subscriptions use the same guard.

```rust
let _guard = store.subscribe(|snapshot| {
    println!("{}", snapshot.context.count);
    Ok(())
}).scoped();
```

`store.select(selector, comparator)` returns a cached `Selection`. Its selector projects only the supplied context; reads captured from other stores are not tracked. `Selection::get` returns a shared value handle. The explicit comparator decides whether a newly computed value replaces the cached one. Selected subscriptions suppress equal results and support the same explicit cleanup.

Selections compose through one explicit dependency using `selection.map(project, comparator)`. Mapping is lazy, caches by the source value's identity, and retains the previous mapped handle when its comparator reports equality. Chaining does not add automatic dependency tracking.

```rust
let count = store.select(|context| context.count, |left, right| left == right);
let parity = count.map(|count| count % 2, |left, right| left == right);
let label = parity.map(|parity| format!("parity: {parity}"), |left, right| left == right);
```

Plain reads remain current without subscribing. Live mapped listeners share an upstream observer attachment, which detaches after the last explicit unsubscribe. Dropping a selection handle or its returned subscription does not cancel registered listeners. Mapped handles retain their source, so they remain usable after the original source handle is dropped.

Dependency reads establish notification ordering before subscriptions become active. An unwatched chain recomputes on its next read, while comparator-equal results keep their cached value handles.

Two declared inputs compose through `left.combine(&right, project, comparator)`. The projector receives both selected values by reference and returns a cached result. Inputs can belong to the same store or independent stores; combining already-derived selections reuses their calculations. Equality retains the previous result handle, duplicate inputs share observation, and common-root diamond updates produce one coherent result. This is explicit dependency composition, not automatic tracking. Separate sends to independent stores remain separate transitions; combining does not create a multi-store transaction.

Selection handles keep their input stores alive. Registered callbacks use internal non-owning readers to avoid cycles between independent stores. If all public selection handles and one input store are dropped while another store keeps a combined registration active, the expired input retains its last materialized selected value. Updates from the surviving store continue against that cached value. Dropping all store owners releases the registration graph; this is a Rust-native ownership boundary rather than a JavaScript garbage-collection guarantee.

Captured strong store or selection handles can form reference cycles in application callbacks. `store.downgrade()` returns a cloneable `WeakStore` that does not keep the store alive. Its `upgrade()` returns an ordinary owning store handle while the store remains alive, or `None` after its final owning handle is dropped. Selections and retained effect scopes intentionally remain owning handles. Weak handles add no context, event or emitted-value trait bounds.

```rust
let weak = store.downgrade();
let _guard = store.subscribe(move |snapshot| {
    if snapshot.context.count == 1 {
        if let Some(store) = weak.upgrade() {
            store.send(Event::Increment(1))?;
        }
    }
    Ok(())
}).scoped();
```

Use weak captures when callbacks should not own their store, and scoped guards or explicit unsubscribe when observation ends. Capturing an owning selection or effect scope can still retain the store.

## Reusing store behavior

`StoreFactory::new(initializer, reducer)` defines reusable behavior. `factory.create(input)` consumes one input, runs the fallible initializer and returns an ordinary store. Unit input `()` covers definitions without configuration.

```rust
use rstate::{StoreFactory, Update};

enum CounterEvent {
    Increment,
}

let counters: StoreFactory<i32, i32, CounterEvent> = StoreFactory::new(
    |initial_count: i32| Ok(initial_count),
    |count, event, _| {
        Ok(match event {
            CounterEvent::Increment => Update::replace(**count + 1),
        })
    },
);

let first = counters.create(0).unwrap();
let second = counters.create(10).unwrap();
first.send(CounterEvent::Increment).unwrap();
assert_eq!(*first.get_snapshot().context, 1);
assert_eq!(*second.get_snapshot().context, 10);
```

Each creation allocates fresh context and snapshot handles with independent subscriptions, emissions and selection caches. An initializer failure returns its `StoreError` before a store is created and leaves the factory reusable. Factories share their initializer and reducer callbacks through `Rc`; captured application state inside those callbacks is intentionally shared. Cloning a factory adds no `Clone` bounds to input, context, event or emitted types, and dropping it does not invalidate stores already created from it.

## Inspecting transitions

`store.inspect(callback)` immediately reports `Inspection::Initial` with the current snapshot, then reports `Inspection::Transition` for each received outer event, including unchanged and unknown events. It returns `StoreResult<Subscription>`; use explicit `unsubscribe` to remove the inspector. Dropping the returned handle leaves registration active.

```rust
use rstate::Inspection;

let inspection = store.inspect(|record| {
    match record {
        Inspection::Initial { snapshot, .. } => {
            println!("initial count: {}", snapshot.context.count);
        }
        Inspection::Transition { snapshot, .. } => {
            println!("transition count: {}", snapshot.context.count);
        }
        Inspection::Control { action, snapshot, .. } => {
            println!("{action:?} count: {}", snapshot.context.count);
        }
        _ => {}
    }
    Ok(())
}).unwrap();

inspection.unsubscribe();
```

A transition record borrows its event for the callback and owns a shared snapshot handle that the callback can retain. Inspection does not require events or contexts to implement `Clone`. The record's snapshot belongs to that dispatch, even if a subscriber or another inspector has synchronously sent a newer event.

State is committed before ordinary and selected subscribers run, followed by inspectors and then effects or emitted events. Subscriber failure skips inspection and effects. Inspector failure stops later inspectors and effects while retaining committed state. Internal queued triggers produce one final outer record; `can` and pure `transition` queries produce no inspection records.

Inspector iteration follows live registration order: removing a listener before its turn skips it, while adding a listener during notification gives it an immediate initial record and then the current transition record. Registration precedes the immediate callback, so a nested dispatch can notify that inspector. If the initial callback returns an error, `inspect` removes only that registration before returning the error. Its captured values are released outside interior borrows; nested state changes and other registrations remain intact. This native cleanup policy intentionally differs from the reference, which retains an inaccessible failed registration. A successful registration keeps ordinary explicit-unsubscribe behavior.

## Reset and snapshot history

`store.reset()` restores the defaults captured when that instance was configured. It creates a fresh snapshot even when the context already equals those defaults, notifies subscribers, and reports `Inspection::Control` with `ControlAction::Reset`. It does not invoke the reducer or execute effects. `reset_with` takes a fallible function receiving the shared default and current context handles and returning the replacement handle; failure leaves live state unchanged.

Enable bounded snapshot history with `StoreConfig::with_history(limit)` or `StoreFactory::with_history(limit)`:

```rust
let history: Store<i32, i32> = Store::new(
    StoreConfig::new(0, |count, by, _| {
        Ok(Update::replace(count.saturating_add(*by)))
    }).with_history(100),
);
history.send(3).unwrap();
assert!(history.can_undo());
history.undo().unwrap();
assert_eq!(*history.get_snapshot().context, 0);
history.redo().unwrap();
history.reset().unwrap();
history.undo().unwrap();
assert_eq!(*history.get_snapshot().context, 3);
```

Every successfully evaluated outer domain event records its pre-state, including no-ops. An internal FIFO macrostep records one entry. New domain events and resets clear redo history. Zero capacity is valid and retains no entries. Undo and redo restore shared context handles without replaying reducers, effects or emissions; snapshot history assumes immutable context replacement and cannot reverse arbitrary interior mutation.

`store.history_status()` and `snapshot.history_status()` expose `HistoryStatus { past, future, limit }`; `None` means history is disabled. History-only changes publish fresh snapshots, so ordinary subscribers can observe availability even when the context handle stays equal. Context selections continue suppressing equal projections. Empty undo/redo operations preserve snapshot identity but still report their typed control action. Domain `can` and pure `transition` queries simulate returned history without mutating live history or storage, and domain availability keeps the original reducer's meaning.

State and history commit together before notifications, followed by inspection, persistence and user effects. Subscriber or inspector errors retain committed history and stop later pipeline stages. Reentrant commits retain each dispatch's inspection snapshot. Reset is undoable when history is enabled.

## Synchronous snapshot persistence

`Persistence::new(name, version, storage, codec)` supplies a synchronous `SnapshotStorage` byte adapter and `SnapshotCodec<Context>`. `Store::new_persisted(config, persistence)` and `factory.create_persisted(input, persistence)` load before exposing the store. Missing data uses configured defaults; storage, decoding and version failures return typed errors without overwriting saved data. Enable the optional `json` feature for `JsonCodec`; other codecs add no global serialization bounds.

The loaded context becomes the visible initial snapshot and empty-history starting point. Configured factory defaults remain the reset target, so resetting a hydrated store restores defaults and undoing that reset can recover the loaded context. Only context is persisted; history stacks and subscriptions are not serialized.

Committed domain events and controls automatically save after successful subscriber/inspection delivery and before user effects. Revision and clear-epoch checks prevent stale outer frames from overwriting newer nested commits or recreating storage after a nested clear. Nested domain sends during codec or storage callbacks are serialized into the latest pending write. Persistence bookkeeping is outside snapshots: attaching persistence alone does not manufacture identity changes for otherwise unchanged domain or empty control operations.

Automatic-save failures leave committed state intact, report through `Persistence::on_error` and `store.persistence_error()`, and do not turn the domain action into a storage error. `store.save()` explicitly retries the current context; `store.clear_storage()` removes saved data without resetting live state or history. Explicit operations return typed errors, including a busy error if called reentrantly from active storage I/O. A successful write clears the last reported persistence error. `save()` propagates a queued write failure and returns `PersistenceError::Superseded` if reentrant changes leave the current revision unpersisted.

## Schemas and validation

`StoreConfig::with_schemas` and `StoreFactory::with_schemas` attach typed `Schemas<Context, Event, Emitted>` metadata. Each optional context, event or emitted `Schema<T>` has a `SchemaMetadata` name and vendor and a synchronous check callback. `store.schemas()` exposes the configured schemas. Attaching schemas is inert: ordinary `Store::new`, factory `create`, and persisted constructors do not run schema checks.

```rust
use rstate::{Schema, SchemaCheck, SchemaMetadata, Schemas, ValidationIssue, ValidationOptions};

let schemas = Schemas::default().with_context(Schema::new(
    SchemaMetadata::new("nonnegative-counter"),
    |context: &Counter| {
        Ok(if context.count >= 0 {
            SchemaCheck::Valid
        } else {
            SchemaCheck::invalid(vec![ValidationIssue::new("count must be nonnegative")])
        })
    },
));
let config = StoreConfig::new(Counter { count: 0 }, |_, _, _| Ok(Update::Keep))
    .with_schemas(schemas);
let validated: Store<Counter, Event> = Store::new_validated(config, ValidationOptions::default()).unwrap();
```

Runtime validation is enabled only through the explicit fallible `Store::new_validated(config, options)`, `Store::new_persisted_validated(config, persistence, options)`, `factory.create_validated(input, options)`, or `factory.create_persisted_validated(input, persistence, options)` constructors. Initialization failures return a typed error before exposing a store. A validated persisted constructor checks configured reset defaults before reading storage, then checks any loaded context before exposure; rejected data is not overwritten. Missing stored data uses the already-validated defaults.

Checks run on the outer incoming event before the reducer, the final macrostep context, and collected emitted values in their existing order before state/history publication or effects. Internal `enqueue.trigger` event payloads bypass event validation, matching the pinned reference, while the final context and collected emissions remain checked. Successful schema outputs do not replace values: the reference also ignores Standard Schema `result.value`. Schema checks preserve context, snapshot, event and emitted identities and add no `Clone` or serialization bounds.

`SchemaCheck::invalid(issues).named(name)` rejects a value with structured issues; `SchemaCheck::unknown(name)` classifies an unsupported event or emitted variant. A callback can return `Valid` for known variants without their own validation rule. `ValidationOptions` defaults to checking every scope and rejecting unknown event/emitted variants; individual scopes can be disabled and either unknown policy can be set to `UnknownPolicy::Ignore`. `ValidationIssue` paths contain explicit `PathSegment::Field` and `PathSegment::Index` values. Returning `Unknown` from a context schema rejects the context.

Validation failures return an opaque `StoreError` containing a `ValidationError` with stage, reason, schema metadata, optional value name and issues. Read it through `StoreError::validation_error()` or `Error::source()`, without retaining the original context or payload. Provider failures retain their displayed message and causal chain; dispatch status describes the operation returning them. `can` returns false for validation failures and propagates other errors. Pure `transition` and `can` run checks but do not commit state/history, notify, inspect, execute effects or write storage; schema callbacks must remain pure like reducers.

For native controls, reset and undo/redo check the proposed context before committing, without inventing a domain event. This protects the validated-store invariant even when schema rules capture changing application configuration. Structural `can_undo` and `can_redo` report history availability, while a subsequent restore can still fail validation. Rejected controls leave current state, history and storage unchanged. Validating configured defaults, hydrated state and native controls is an explicit native composition policy; arbitrary upstream extension ordering is not reproduced. Async validators and JSON Schema interpretation are outside this typed synchronous interface.

## Layout

`crates/rstate/src` composes focused state, transition, effects, observation, store, control, history, persistence, schema and validation domains. Native behavior tests live in the separate mirrored `crates/rstate/tests` tree and use a single explicit Cargo integration-test target. No handwritten code comments or inline test modules are used.

`crates/rstate-wasm` is an unpublished browser test adapter. `test-app` runs real Rust/WASM and the pinned JavaScript reference over matching scenarios. It is development tooling, not a general JavaScript SDK.

`conformance` owns shared scenario fixtures and their compatibility contract.

## Development

Use the installed stable Rust toolchain with the WASM target. The initial implementation was checked with Rust 1.98.0. The manifest uses Rust edition 2024 and declares Rust 1.85 as its language baseline; the minimum compiler has not been separately tested.

```sh
cargo test --locked --workspace --all-features
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo clippy --locked -p rstate-wasm --target wasm32-unknown-unknown --all-features -- -D warnings
```

Unsafe code is forbidden. Clippy's `all`, `pedantic` and `nursery` groups are denied, together with explicit restrictions on unwraps, expects, panics, indexing, unchecked arithmetic, casts and unfinished code. The upstream allowances for repeated module names, suggested `must_use` attributes and error/panic documentation are retained; usage and rationale live in Markdown.

The root `clippy.toml` also follows upstream: unwraps, expects, indexing and panics are permitted inside tests. Production code retains those restrictions. Any additional exception must name the individual lint, stay narrowly scoped and include a reason. GitHub Actions runs the formatting, native and WASM lint checks and native tests above on pull requests and pushes to `main`.

See the test app's README for browser setup, building the WASM adapter and running parity tests.

## Current boundary

The core is intentionally single-threaded and synchronous. Selection composition uses explicit one- or two-source dependencies; automatic dependency discovery and general atom APIs remain outside the implementation scope. Async/source atoms, managed async effects and XState actor integration remain outside scope. History uses bounded snapshots; event replay, grouping, filters and custom restore effects are deferred. Persistence saves full context synchronously; async storage, event logs, partial merges, migrations, throttling and broadcast are deferred. Validation uses typed synchronous callbacks; automatic value transformation, asynchronous validators and a general schema language are deferred. Transition inspection emits callbacks only and does not implement the XState actor inspection protocol.
