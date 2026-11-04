# rstate-gpui

The adapter connects rState selections and persistence feedback to Zed GPUI views, and owns asynchronous application work through explicit command scopes. Domain dispatch remains synchronous. Changed bindings queue a detached foreground task that notifies the consuming entity after its current handler returns. Several changed bindings sharing one target produce one pending notification. Reads return current shared values; queued invalidation never captures a snapshot for rendering.

This package uses exactly `gpui` 0.2.2 from crates.io. It is a separate desktop workspace, verified with Rust 1.98 on Linux, and is not published. The core workspace retains its Rust 1.85 baseline and WASM boundary. The different GPUI distributions used by gpuikit and Longbridge gpui-kit have distinct entity and context types and are not supported by this package.

Consumers use the same `gpui = "=0.2.2"` package when constructing entities and contexts for the adapter. The adapter's Linux manifest also constrains `libc` to `=0.2.189`: GPUI 0.2.2 indirectly depends on `xattr` 0.2.3, which references the compatibility constant `libc::ENOATTR` removed in newer libc releases. This constraint participates in a consumer's dependency resolution; copying this repository's lockfile is unnecessary. It also means consumers requiring a conflicting libc release need an engine/dependency upgrade rather than a second incompatible lockfile. The core has no libc or GPUI dependency.

## Bind selected state

Construct one `ViewBindings<Self>` inside an entity initializer and reuse it for that entity's bindings. It can be cloned without bounds on the view type; its clones share the same notification target. Creating multiple independent targets for one entity prevents coalescing between those targets. Keep bindings in the view, and recreate them deliberately when replacing its document dependency.

```rust
let bindings = ViewBindings::new(cx);
let title = bindings.bind(store.select(
    |context| context.title.clone(),
    PartialEq::eq,
));
let can_undo = bindings.bind(store.select_snapshot(
    |snapshot| snapshot.history_status().past != 0,
    PartialEq::eq,
));
```

`SelectionBinding::get()` returns the underlying selection's current `Rc<T>`, including before any event. Subscriptions do not replay an initial value. Comparator-equal changes retain selected value identity and schedule no notification. Existing `map` and `combine` selections can be bound directly. Contexts and selected values need no `Clone`, `Debug`, `Send`, `Sync` or serialization bounds.

Bindings are not cloneable. Each owns its selection and a scoped subscription. Dropping a binding invalidates its registration before unsubscribing; stale queued work cannot activate a replacement registration. A queued job still delivers dirty work for other live bindings. Targets retain only a weak GPUI entity. A released entity is ordinary cleanup and never turns a successful domain action into a subscriber error. Binding callbacks retain notification metadata, without retaining the selection or store themselves. An externally retained binding intentionally keeps its source store alive.

The notification task is detached immediately, so it survives subscriber return. It drains dirty registrations and clears pending state before entering GPUI. Changes during notification can queue another task. Notification coalescing changes only view invalidation; events, nested dispatch, effects, emissions and inspection retain their core ordering. It does not promise one GPU draw or a particular frame deadline.

## Dispatch errors

`dispatch(&store, event, on_error)` calls `Store::send` immediately and returns `true` on success. On failure it passes the original `StoreError` to `on_error` exactly once and returns `false`. `handle_result(result, on_error)` handles the same routing for existing control methods such as `undo`, `redo` and `reset`.

```rust
dispatch(&store, Event::Rename(title), |error| {
    self.error = Some(error.to_string());
});
handle_result(store.undo(), |error| {
    self.error = Some(error.to_string());
});
```

A committed error preserves the committed state. A not-committed reporting call can still contain nested commits. Neither result triggers retries or rollback. Dispatch success does not prove persistence succeeded; automatic persistence failures use the core's separate reporting path. History bindings describe structural availability; validation can still reject a restore, so control results must be handled.

## Persistence feedback

`bindings.bind_persistence(&store)` returns `StoreResult<PersistenceBinding>` for a persistence-configured store. Its `get()` reads the current shared core `PersistenceStatus`, including before the first save. Bind once during initialization and retain the non-cloneable guard for the desired observation lifetime.

```rust
let feedback = bindings.bind_persistence(&store)?;
let status = feedback.get();
let last_activity = status.last_activity();
let last_outcome = status.outcome();
```

Persistence and selection bindings created from the same `ViewBindings` share its notification target, so one operation changing both domain state and persistence feedback still has one pending view invalidation. Explicit save or clear can update feedback without publishing a domain snapshot. Dropping the binding invalidates queued work before unsubscribing; a released entity remains ordinary cleanup. The binding intentionally owns its store, while its subscriber captures only target and registration metadata.

The core distinguishes last activity, returned outcome and the storage error captured when that status was recorded. In-flight explicit operations can expose a newer live `persistence_error()` before recording their completed status. Failed saves, recovery, rejected reentrant calls, supersession and clear operations remain typed. Status is a latest observation with potentially coalesced sequence numbers; a successful clear or absent error does not establish that current context is saved. Feedback uses the existing synchronous core persistence API and adds no async storage or serialization bounds. See the [core persistence contract](../../README.md#synchronous-snapshot-persistence).

## Commands and task ownership

`CommandScope::new(cx)` owns work for an application, manager, operation group or view. Retain the non-cloneable scope for that lifetime. Create a `CommandChannel` for each operation key; clones of a channel share its policy and lane. New channels are independent, including when they use the same policy. Channels and returned `CommandTicket` handles do not keep their scope alive.

```rust
let commands = CommandScope::new(cx);
let calculations = commands.channel(CommandPolicy::Latest);
let weak = store.downgrade();

let ticket = calculations.submit_background(
    move || async move { calculate(request) },
    move |result, _cx| {
        if let Some(store) = weak.upgrade() {
            store.send(Event::Calculated(result))
        } else {
            Ok(())
        }
    },
    move |error, _cx| report_delivery_error(error),
)?;
```

Submission only records a command and schedules foreground startup after the current GPUI handler or synchronous store callback returns. Eligible foreground factories and background-job submissions follow submission order; a blocked serial lane does not block independent channels. GPUI's background executor can invoke and poll concurrent background factories out of order. Foreground futures may also be polled, and concurrent work may complete, in a different order. A serial channel waits for completion and delivery before submitting its next job. Every command is retained for its scope's lifetime unless its caller cancels or replaces it; redraw coalescing does not discard command occurrences.

| Policy | Behavior |
| --- | --- |
| `Concurrent` | Start each submitted command without waiting for another command's result. |
| `Serial` | Start the next factory after the previous work and its result/error delivery finish. Use this for ordered operations such as writes. |
| `Latest` | Cancel earlier queued or running requests on that channel and accept only the current request's result. Use this for replaceable operations such as searches. |

`channel.submit(factory, deliver, on_error)` passes an owned `AsyncApp` to the foreground factory and accepts non-`Send` futures and results. `submit_background` accepts a factory without an app context and runs its future on GPUI's background executor. Only that background factory, its future and its result require `Send`. Contexts, stores, delivery callbacks and foreground selected values remain on the UI thread without new `Clone`, serialization or thread-safety bounds. The adapter adds no async runtime; work must be compatible with its selected executor and any application-provided I/O integration.

`deliver(result, &mut App)` runs on the UI thread and returns `StoreResult<()>`. Expected operation failures can be part of the application's typed result, such as `Result<Response, RequestError>`, and are delivered normally. A failure returned while publishing that result is passed unchanged to `on_error(StoreError, &mut App)` exactly once, including the original dispatch status. The bridge never retries or rolls back a domain event. A released app suppresses delivery during ordinary cleanup.

Request IDs increase within their scope and live outside domain snapshots and undo history. `ticket.cancel()` invalidates one request; `channel.invalidate()` invalidates an operation lane; `scope.invalidate()` invalidates all current work while leaving the scope reusable. Call these explicitly when inputs, application reset or lifetime rules make old outcomes obsolete. `scope.close()` permanently rejects new submissions, and dropping a scope closes it. Submission after closure or owner release returns `CommandSubmitError::Closed`.

Dropping an ordinary ticket does not cancel its command. `ticket.is_pending()` reports eligibility for result delivery, so cancellation makes it false immediately. For an already-started `Serial` command, cancellation suppresses delivery while the owner continues awaiting its work before starting another command. This also covers a factory that invalidates its own request after starting background work. Queued serial commands cancel immediately. Closing or dropping the whole scope cancels its remaining task handles and starts no further work. Cancellation does not promise that blocking OS calls or external side effects are reversed, or that different scopes serialize access to the same resource.

## Build and verify

Run these commands from the repository root. GPUI on Linux needs its native build libraries, including fontconfig, FreeType, xkbcommon, X11/XCB, Wayland and Vulkan. Platform requirements follow the selected [GPUI release](https://docs.rs/crate/gpui/0.2.2/source/README.md). This integration is currently verified on Linux; macOS and Windows are not verified.

On Debian/Ubuntu, install the same development packages as desktop CI. `libxkbcommon-x11-dev` supplies the unversioned linker library; the runtime package alone is insufficient.

```sh
sudo apt-get update
sudo apt-get install -y libclang-dev libfontconfig1-dev libssl-dev libwayland-dev libxkbcommon-x11-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libvulkan-dev
```

```sh
CARGO_BUILD_JOBS=2 cargo test --manifest-path desktop/Cargo.toml --locked --workspace --all-targets --all-features
cargo fmt --manifest-path desktop/Cargo.toml --all -- --check
CARGO_BUILD_JOBS=2 cargo clippy --manifest-path desktop/Cargo.toml --locked --workspace --all-targets --all-features -- -D warnings
python3 desktop/scripts/check-consumer.py
CARGO_BUILD_JOBS=2 cargo run --manifest-path desktop/Cargo.toml --locked -p rstate-gpui --example documents
```

The consumer check copies only the standalone fixture's manifest and source into a temporary workspace, generates its own lockfile, verifies one matching GPUI package identity plus the compatible libc release, and checks the public API from that consumer. It reuses compiled artifacts in `desktop/target` but never copies a repository lockfile or adds a dependency patch. Pass `--offline` when the required registry data and packages are already cached. Desktop CI runs fresh resolution to catch changes allowed by upstream dependency constraints.

The deterministic tests use actual GPUI entities, observers and its foreground test scheduler. They cover notification lifetime and coalescing, current reads, identity, replacement and release, nested dispatch, history metadata, trait bounds, error routing and store cleanup. The document example supplies a native rendering and action smoke check; see [the example guide](examples/documents/README.md).

Command scopes and persistence bindings are reusable library infrastructure verified through focused generic fixtures. The existing document example remains a foundation regression fixture. Optional inspector tooling remains a later stage described in [GPUI_ADAPTER.md](../../GPUI_ADAPTER.md). The adapter adds no async runtime or automatic dependency tracking to the core.
