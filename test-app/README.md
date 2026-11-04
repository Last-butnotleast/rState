# Core lab

The lab executes the native rstate core through a private WebAssembly adapter and compares it with the pinned `@xstate/store` reference. React renders controls and traces; state changes run in Rust.

## Run

Prerequisites are Rust with `wasm32-unknown-unknown`, `wasm-pack`, Node 24 and npm 11. The verified environment uses Rust 1.98.0, wasm-pack 0.13.1, Node 24.10.0 and npm 11.6.1.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --version 0.13.1 --locked
cd test-app
npm ci
npm run dev
```

Open the local URL printed by Vite, normally `http://127.0.0.1:4173`. Engine preparation verifies the reference cache and builds the Rust adapter before starting the app.

## Verify

```sh
npm run typecheck
npm run build
npx playwright install chromium
npm test
```

`npm test` builds the engines and runs Chromium tests. `npm run test:prepared` runs the browser suite without rebuilding unchanged engines. Native library tests run separately with `cargo test -p rstate` from the repository root.

The suite covers synchronous callback timing, unchanged and replaced snapshot identities, capability checks, pure transitions, queued internal events, effect and emission order, callback reentry, recoverable errors, selectors, unsubscribe and UI teardown. Factory checks create two input-initialized instances from one reusable definition, release the factory, then verify independent state, observations, selections and emissions against the upstream factory.

## Reference and traces

`reference.lock.json` pins commit `9b3a88fa43f4be5433797acfba18f27c40494bd2`, package version 4.3.0, source checksums and the MIT license. `npm run reference` downloads only the source files needed by the oracle and verifies cached files before use. The generated cache is ignored.

Shared declarative scenarios live in `../conformance/scenarios`. Rust fixture reducers are in `../crates/rstate-wasm/src/scenarios`; equivalent reference handlers are in `src/engines/reference.ts`. The common runner compares ordered entries and identity relationships, beyond final JSON values. Identity IDs are allocated from actual retained Rust handles and JavaScript object references.

The original three fixtures cover a counter, cart composition and nested effect dispatch. `factory.json` adds a separate small two-store scenario with input values and store-tagged operations. Its runner uses one actual Rust `StoreFactory` and one upstream `createStoreLogic` definition, rather than separately constructed seeded stores. The live factory panel lets each input-created counter evolve independently and compares the shared factory trace.

`selection.json` checks an explicit source → bucket → label chain. Rust calls `Selection::map` twice; the oracle uses two actual computed `createAtom` values reading the source selection. The trace compares lazy projector counts, per-selection retained identities, equality suppression, shared listeners, detachment and reactivation. Additional browser checks cover nested sends and callback errors. The selection panel displays the live chain and its calculation counts.

This is a test bridge for these fixtures, rather than a general JavaScript API for arbitrary Rust stores. Recoverable fixture errors use explicit library errors; Rust panics and WebAssembly traps are outside the compatibility contract. General atoms, async runtimes and extensions are outside the core scope.

## Layout

Application state and operations live in `src/app/session.ts` and `useLab.ts`; the display receives values and callbacks. The factory and selection panels follow the same arrangement under `src/app/factory` and `src/app/selection`. Engine adapters, scenarios and trace comparison have their own domains. Tests live in a separate tree under `tests`. Generated WASM bindings live in `src/generated` and are ignored.
