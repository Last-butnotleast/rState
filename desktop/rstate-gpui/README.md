# rstate-gpui

The foundation adapter connects rState selections to Zed GPUI views. Domain dispatch remains synchronous. Selected changes queue a detached foreground task that notifies the consuming entity after its current handler returns. Several changed bindings sharing one target produce one pending notification. Reads return the live selected `Rc<T>`; queued work never captures a snapshot for rendering.

This package uses exactly `gpui` 0.2.2 from crates.io. It is a separate desktop workspace, verified with Rust 1.98 on Linux, and is not published. The core workspace retains its Rust 1.85 baseline and WASM boundary. The different GPUI distributions used by gpuikit and Longbridge gpui-kit have distinct entity and context types and are not supported by this package.

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
CARGO_BUILD_JOBS=2 cargo run --manifest-path desktop/Cargo.toml --locked -p rstate-gpui --example documents
```

The desktop lockfile deliberately selects `libc` 0.2.189. GPUI 0.2.2 depends indirectly on `xattr` 0.2.3, which references `libc::ENOATTR`; libc 0.2.191 removes that compatibility constant on Linux. Until GPUI's transitive dependency is upgraded, external workspaces using this engine need a compatible libc lockfile too. Reproduce this repository's desktop build with `--locked`; do not copy or patch dependency source files.

The deterministic tests use actual GPUI entities, observers and its foreground test scheduler. They cover notification lifetime and coalescing, current reads, identity, replacement and release, nested dispatch, history metadata, trait bounds, error routing and store cleanup. The document example supplies a native rendering and action smoke check; see [the example guide](examples/documents/README.md).

Background command tasks, cancellation and stale-result handling, observable persistence status and the optional inspector remain later stages described in [GPUI_ADAPTER.md](../../GPUI_ADAPTER.md). The foundation adds no async runtime or automatic dependency tracking to the core.
