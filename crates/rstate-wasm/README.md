# Browser conformance bridge

This private crate connects fixed test scenarios to the actual rstate library. It owns browser serialization, JavaScript callbacks and identity transport; the core crate has no browser or serialization dependency.

`FixtureStore` exposes shared-borrow operations so JavaScript callbacks can synchronously call back into Rust. Interior borrows are released before invoking callbacks. Snapshot and context handles remain retained while the fixture is alive, giving identity comparisons stable IDs without pointer reuse.

`FixtureFactory` holds one actual `StoreFactory` definition. Each `create` consumes a JSON input containing an initial count and returns a separate `FixtureStore` using the cart-composition reducer. The factory can be freed while its stores continue operating. The matching browser oracle reuses one upstream `createStoreLogic` definition.

`FixtureSelection` owns a count selection and two explicit `Selection::map` stages. It exposes reads, projector counts and subscriptions for the fixed source → bucket → label fixture. Per-target IDs retain actual Rust value handles, and JavaScript callbacks run after identity-table borrows end. The oracle uses the pinned source's computed atoms for the same chain.

Build and test through `test-app` scripts. Its README contains the verified toolchain and commands. The bridge is not published and is not a general-purpose JavaScript SDK.
