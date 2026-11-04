# rstate

A small synchronous Rust state store with typed events, shared snapshots, pure transitions, subscriptions, effects, emitted events, transition inspection, context selections and reusable factories. The library uses the Rust standard library and `thiserror`; JavaScript, serialization and browser dependencies stay in the separate test adapter.

The selected behavior follows [`@xstate/store` 4.3.0 at commit `9b3a88f`](https://github.com/statelyai/xstate/tree/9b3a88fa43f4be5433797acfba18f27c40494bd2/packages/xstate-store). This is a core implementation, not a port of the complete package or its extensions.

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
        Inspection::Initial { snapshot } => {
            println!("initial count: {}", snapshot.context.count);
        }
        Inspection::Transition { snapshot, .. } => {
            println!("transition count: {}", snapshot.context.count);
        }
    }
    Ok(())
}).unwrap();

inspection.unsubscribe();
```

A transition record borrows its event for the callback and owns a shared snapshot handle that the callback can retain. Inspection does not require events or contexts to implement `Clone`. The record's snapshot belongs to that dispatch, even if a subscriber or another inspector has synchronously sent a newer event.

State is committed before ordinary and selected subscribers run, followed by inspectors and then effects or emitted events. Subscriber failure skips inspection and effects. Inspector failure stops later inspectors and effects while retaining committed state. Internal queued triggers produce one final outer record; `can` and pure `transition` queries produce no inspection records.

Inspector iteration follows live registration order: removing a listener before its turn skips it, while adding a listener during notification gives it an immediate initial record and then the current transition record. Registration precedes the immediate callback. If that callback fails, `inspect` returns its error while the registration remains active and no unsubscribe handle is returned, matching the reference behavior.

## Layout

`crates/rstate/src` composes focused files under five domains: state, transition, effects, observation and store. Native behavior tests live in the separate mirrored `crates/rstate/tests` tree and use a single explicit Cargo integration-test target. No handwritten code comments or inline test modules are used.

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

The core is intentionally single-threaded and synchronous. Selection composition uses explicit one- or two-source dependencies; automatic dependency discovery and general atom APIs remain outside the implementation scope. Async/source atoms, managed async effects, persistence, reset/undo, validation and XState actor integration are also outside scope. Transition inspection emits callbacks only; it does not retain history or implement the XState actor inspection protocol.
