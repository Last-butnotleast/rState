# rstate

A small synchronous Rust state store with typed events, shared snapshots, pure transitions, subscriptions, effects, emitted events and context selections. The library uses the Rust standard library and `thiserror`; JavaScript, serialization and browser dependencies stay in the separate test adapter.

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

Reducers return `StoreResult<Update<Context>>`. Use `StoreError::new` for a callback failure or `StoreError::from_error` to retain an application error as its source. Callback failures are recoverable results; Rust panics are programming faults rather than JavaScript-compatible exceptions.

`Update::Keep` retains context and snapshot identity. `Update::Replace` replaces the complete context; equal contents do not imply equal identity. Returning the existing context handle preserves snapshot identity while making `can` true. Contexts, events and selected values do not need `Clone`, `Serialize`, `Send` or `Sync`.

`store.transition(snapshot, event)` returns a proposed snapshot, ordered effects and availability without publishing or running effects. `store.can(event)` evaluates the same reducer and discards its proposed changes and effects. Keep application side effects inside `enqueue.effect`, because reducer code itself runs during these queries.

`enqueue.trigger(event)` processes internal events FIFO within one transition and publishes only its final snapshot. `enqueue.emit(message)` and `enqueue.effect(callback)` retain their collection order. Callback calls to `store.send` reenter synchronously. Shared notification scheduling reconciles nested updates across stores before nested sends return.

Effects receive a cloneable `EffectScope` with `send` and `get_snapshot`. Snapshot reads use that dispatch's snapshot during its entire effect loop, then read the current store after successful completion. If an effect fails, retained scopes remain pinned to that dispatch's snapshot, matching the reference.

Subscriptions do not replay their initial value. Dropping a subscription handle leaves registration active; call `unsubscribe` explicitly. It is idempotent. A subscriber failure preserves committed state, drains remaining scheduled notifications, returns the first failure and skips that dispatch's effects. Emitted listeners and effects fail immediately, retaining committed state. Filtered emitted listeners run before wildcard listeners.

`store.select(selector, comparator)` returns a cached `Selection`. Its selector projects only the supplied context; reads captured from other stores are not tracked. `Selection::get` returns a shared value handle. The explicit comparator decides whether a newly computed value replaces the cached one. Selected subscriptions suppress equal results and support the same explicit cleanup.

Captured strong store or selection handles can form reference cycles in application callbacks. Explicitly unsubscribe when those callbacks are no longer needed.

## Layout

`crates/rstate/src` composes focused files under five domains: state, transition, effects, observation and store. Native behavior tests live in the separate mirrored `crates/rstate/tests` tree and use a single explicit Cargo integration-test target. No handwritten code comments or inline test modules are used.

`crates/rstate-wasm` is an unpublished browser test adapter. `test-app` runs real Rust/WASM and the pinned JavaScript reference over matching scenarios. It is development tooling, not a general JavaScript SDK.

`conformance` owns shared scenario fixtures and their compatibility contract.

## Development

Use the installed stable Rust toolchain with the WASM target. The initial implementation was checked with Rust 1.98.0. The manifest uses Rust edition 2024 and declares Rust 1.85 as its language baseline; the minimum compiler has not been separately tested.

```sh
cargo test -p rstate
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

See the test app's README for browser setup, building the WASM adapter and running parity tests.

## Current boundary

The core is intentionally single-threaded and synchronous. General atoms, cross-store reactive dependency graphs, async/source atoms, managed async effects, persistence, reset/undo, validation, inspection, reusable factories and XState actor integration are outside the implementation scope.
