# Browser conformance bridge

This private crate connects fixed test scenarios to the actual rstate library. It owns browser serialization, JavaScript callbacks and identity transport; the core crate has no browser or mandatory serialization dependency. This adapter enables the core's optional `json` codec feature.

`FixtureStore` exposes shared-borrow operations so JavaScript callbacks can synchronously call back into Rust. Interior borrows are released before invoking callbacks. Snapshot and context handles remain retained while the fixture is alive, giving identity comparisons stable IDs without pointer reuse.

`FixtureFactory` holds one actual `StoreFactory` definition. Each `create` consumes a JSON input containing an initial count and returns a separate `FixtureStore` using the cart-composition reducer. The factory can be freed while its stores continue operating. The matching browser oracle reuses one upstream `createStoreLogic` definition.

`FixtureSelection` owns a count selection and two explicit `Selection::map` stages. It exposes reads, projector counts and subscriptions for the fixed source → bucket → label fixture. Per-target IDs retain actual Rust value handles, and JavaScript callbacks run after identity-table borrows end. The oracle uses the pinned source's computed atoms for the same chain.

Build and test through `test-app` scripts. Its README contains the verified toolchain and commands. The bridge is not published and is not a general-purpose JavaScript SDK.

`FixtureStore::inspect` forwards real core inspection records with retained snapshot identities. Inspection serializes the typed fixture event rather than retaining arbitrary JavaScript input objects. Inspection scenarios therefore use explicit numeric payloads and the fixture's exact `unknown` event tag; optional-field spelling and arbitrary unknown event names are outside that fixture contract. Inspector callbacks may synchronously register other observers, so subscription identifiers are reserved before callback delivery.

`FixtureCombined` uses the public two-input `Selection::combine` API and a mapped label. Its fixed modes cover independently owned stores, a diamond with two mapped branches of one source, and the same selection supplied twice. Values and projector counts remain lazy; retained per-target handles expose identity relationships. The reference reads the same two inputs through an actual upstream computed atom. The fixture projects `floor((left + right) / 2)` to exercise equality suppression, and numeric inputs remain small and nonnegative. Separate sends to independent stores remain separate commits.

`FixtureControls` configures the actual reset/history APIs and optionally constructs a persisted store with `JsonCodec` and synchronous JavaScript storage callbacks. Its base fixture shares the real store and exposes retained snapshot identities plus history status. Inspectors serialize `ControlAction` records as reset/undo/redo event names without fabricating domain events. The matching oracle uses the pinned reset, snapshot undo and snapshot persistence extensions. Storage callback borrows end before entering JavaScript, allowing subscriber/inspector reentry.

The controls fixture retains configured reset defaults separately from hydrated initial state. It persists context and version only. Automatic storage failures expose their original causal message through the fixture error channel, while corrupt input/version mismatches and explicit save/clear failures remain typed native errors. Native-only policy tests distinguish force-save from upstream pending-write flush and unchanged snapshots from upstream persistence metadata allocation. This adapter remains a fixed conformance fixture, not a generic extension SDK.
