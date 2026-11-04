# Shared documents example

From the repository root:

```sh
cargo run --locked --manifest-path desktop/Cargo.toml -p rstate-gpui --example documents
```

The app opens three native windows. The shared list and detail windows observe one document store. The independent list uses another store created by the same `StoreFactory`.

Click a list item to select it. **Rename (+ *)** appends ` *` to its title; **Delete** removes it and selects the first remaining item when necessary. Both shared windows update from the same domain state. **Undo**, **Redo** and **Reset** use the store's native snapshot controls. Reset restores all three initial items and is itself undoable. Button availability comes from snapshot selections, so a history-only change can update Undo without changing the displayed item.

Each window constructs its bindings once and retains them for its lifetime. Closing the shared list leaves the detail usable; closing both releases that document. The independent document remains usable. Operation errors appear below the controls and retain the core's dispatch status. The example does not retry failed events.

`domain.rs` contains the same factory and transitions used by the headless native tests. `view.rs` contains GPUI presentation and routes typed actions through the adapter. Background document operations and persistence feedback belong to the next design stage.

Run the native window smoke check in a working desktop session:

```sh
cargo run --locked --manifest-path desktop/Cargo.toml -p rstate-gpui --example documents -- --smoke
```

The smoke check opens real windows, allows the foreground executor to render, invokes the same view actions as the controls, checks shared updates and document independence, closes shared windows in sequence, and verifies that the final owner releases its store. It prints a success message and quits, or returns an error. Its short executor timers allow native frames to run; deterministic binding counts and cleanup are covered separately by GPUI scheduler tests.
