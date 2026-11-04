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
