# Browser conformance bridge

This private crate connects fixed test scenarios to the actual rstate library. It owns browser serialization, JavaScript callbacks and identity transport; the core crate has no browser or serialization dependency.

`FixtureStore` exposes shared-borrow operations so JavaScript callbacks can synchronously call back into Rust. Interior borrows are released before invoking callbacks. Snapshot and context handles remain retained while the fixture is alive, giving identity comparisons stable IDs without pointer reuse.

`FixtureFactory` holds one actual `StoreFactory` definition. Each `create` consumes a JSON input containing an initial count and returns a separate `FixtureStore` using the cart-composition reducer. The factory can be freed while its stores continue operating. The matching browser oracle reuses one upstream `createStoreLogic` definition.

Build and test through `test-app` scripts. Its README contains the verified toolchain and commands. The bridge is not published and is not a general-purpose JavaScript SDK.
