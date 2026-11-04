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

This is a test bridge for these fixtures, rather than a general JavaScript API for arbitrary Rust stores. Recoverable fixture errors use explicit library errors; Rust panics and WebAssembly traps are outside the compatibility contract. General atoms, async runtimes and arbitrary extension frameworks are outside the core scope.

`inspection.json` exercises immediate current-state inspection, unchanged outer events, dry-run silence, notification/effect ordering, reentry and unsubscribe. The adapters use real Rust inspection records and the pinned store's `inspect` callback. Snapshot identities are retained; actor/session metadata is outside the native contract. Dedicated browser tests also cover live inspector registration/removal and fail-fast transition errors. Failed initial registration is an explicit native policy difference: Rust removes that registration before returning its error, while the upstream store retains it without returning an unsubscribe handle. Both register before the initial callback, so a synchronous nested send can reach the pending inspector; native cleanup preserves nested commits and independently registered inspectors. Inspected fixture events use explicit numeric payloads and the exact `unknown` tag because the Rust bridge exposes a typed event, not arbitrary original JavaScript objects.

The bridge copies `StoreError::dispatch_status()` into a JavaScript error's `dispatchStatus` property as `notCommitted` or `committed`. This classifies the failed native send or control call's own commit boundary, including a commit that retains snapshot identity. It does not imply that nested sends or callback side effects were rolled back. Query, construction and initial-inspection errors have no dispatch status. Dedicated native-policy browser tests assert actual Rust metadata and retained state for reducer/validation rejection, notification, inspection, effect and emitted-listener failures, nested dispatch and history controls. Common parity traces continue comparing the original causal messages; they do not invent equivalent metadata for upstream errors.

`combine.json` compares explicit two-input computed state in independent-store, shared-source diamond, and duplicate-input modes. The Rust adapter calls `Selection::combine` and then `map`; the reference uses actual computed atoms reading both selections. Traces retain values, identities, projector counts, listener order, and observation lifecycle. Dedicated tests also verify one coherent diamond result, separate independent-store commits, equality retention, nested sends and callback-error draining. This fixture is exercised through the adapters rather than adding another UI panel. Weak handles and scoped subscriptions are native ownership features and are validated by the Rust tests.

`controls.json` compares reset and bounded snapshot undo/redo through real Rust control methods and the official reset/snapshot-undo extensions. `persistence.json` adds actual synchronous persistence, initialized from a versioned context envelope. Ordered traces retain history counts, identities, control inspection, notifications, effects and storage calls. Additional tests cover reentrant writes/clear, failure reporting and a real localStorage page reload. These fixtures have no new UI panel.

Native differences are explicit assertions: domain `can` remains correct, empty controls and persistence bookkeeping preserve unchanged snapshot identity, corrupt/version-mismatched loads reject instead of silently falling back, and `save()` retries the current context. Upstream `flushStorage` flushes pending work and is not presented as equivalent to force-save. Auto-save errors report the original causal message without failing the committed domain action. Only context/version is persisted; history starts empty after reload, and reset returns the configured defaults.

`validation.json` compares schema metadata and explicitly enabled synchronous validation through actual `validateSchemas` and native validated constructors. Fixed Standard Schema adapters log the values checked by the reference; typed Rust checks log the equivalent values. Traces compare rejection reasons, names, issue messages and field/index paths, together with state identities, callback order and macrostep behavior. The fixture declares schemas for each supported event variant and leaves only the exact `unknown` tag undeclared.

Validation is check-only. One test deliberately returns altered Standard Schema success values and proves the upstream extension ignores them. Internal trigger payloads bypass event validation, while the final macrostep context and collected emitted messages are validated before publication. Metadata-only stores never execute checks. Additional browser tests cover options, unknown-name policies, history/control rejection and persistence hydration. Native constructors also validate configured reset defaults before loading persisted data; this stronger reset-safety rule is tested separately from extension-order-dependent upstream behavior. Structured validation fields are compared across languages; native diagnostic stage and schema metadata remain available on the bridged error without rewriting its display message.

## Layout

Application state and operations live in `src/app/session.ts` and `useLab.ts`; the display receives values and callbacks. The factory and selection panels follow the same arrangement under `src/app/factory` and `src/app/selection`. Engine adapters, scenarios and trace comparison have their own domains. Tests live in a separate tree under `tests`. Generated WASM bindings live in `src/generated` and are ignored.
