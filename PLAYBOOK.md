# rState development playbook

Use this playbook to continue the same development method in a fresh session: small agreed increments, delegated implementation, real upstream comparisons, and independent review. It records the workflow and the verified checkpoint at `6a59c23`; it does not authorize additional features or freeze future test counts.

Read [AGENTS.md](AGENTS.md) first. Current user instructions and applicable repository instructions take precedence over this historical handoff. [README.md](README.md) is the API and behavior guide.

## Start a session

1. Read `AGENTS.md`, this playbook, and the relevant README sections.
2. Inspect the current worktree and recent commits before assigning files. Preserve existing edits.
3. Read the installed Cognee memory skill and retrieve a focused knowledge brief before substantive planning, implementation, or review.
4. Confirm the requested scope, installed package/tool versions, and relevant upstream source. Do not implement the next export merely because upstream has it.
5. Agree the minimal API, behavior, native differences, file ownership, and acceptance tests before code changes.

```sh
git status --short
git log -10 --oneline
bash "$HOME/.agents/skills/cognee-memory/scripts/cognee.sh" search "Focused question about the requested change" 5
```

The helper discovers the installed plugin and searches `software_engineering`, dataset `39afdfe2-e6f3-500a-ae40-b13833cf99ff`, using the graph with the explicit dataset ID. Keep approved preferences (`user_context`) and technical references (`project_docs`) available. Do not pin a cached plugin version in project commands.

Keep recalled personal preferences separate from sourced technical guidance. Preserve their scope and caveats; unrelated frontend, database, or framework references are not Rust authority. A relevant parent brief can avoid repeated searches. If recall is irrelevant or unavailable, say so briefly and use current instructions, repository evidence, and primary documentation.

Knowledge access stays read-only. Keep automatic capture disabled. Do not upload conversations, sync sessions, cognify, create datasets, or publish knowledge without a separate explicit request. Writing this playbook does not authorize memory publication.

## Agent workflow

Preserve this user's project preference: the primary agent orchestrates and delegates feature implementation. The primary handles coordination, check results, and requested Git operations. Implementation goes to `coding` agents; a separate `review` agent reviews the finished change. Continue already-authorized work without repeated user approval gates. Agents do not recursively delegate.

With four concurrent slots including the primary, the usual split was:

| Agent | Ownership |
| --- | --- |
| Core implementation | Rust API, focused source modules, necessary wiring and agreed project documentation |
| Native tests | Separate mirrored tests and source-grounded behavioral regressions |
| Browser parity | WASM fixture adapter, actual JavaScript reference, shared scenarios and browser tests |

For larger changes, we split independent Rust domains instead, with each author owning their native tests. One finished agent freed a slot for independent review while the browser work continued.

Every assignment states concrete files/responsibilities, scope, acceptance criteria, reference pin, relevant knowledge brief, and required checks. Tell workers they are not alone in the codebase: preserve others' edits and coordinate contract changes. Assign one owner for shared export files, test entrypoints, manifests, and README sections.

Use this sequence:

1. **Plan.** The core author inspects current code and upstream source, proposes the API and semantics, then sends the contract to the primary and dependent authors. The primary's go-ahead is internal coordination, not another user permission gate.
2. **Implement in useful increments.** Keep the foundation compiling, announce API/source readiness, and continue through the authorized outcome. Do not stop at a scaffold.
3. **Test behavior.** Add native tests and targeted real Rust/WASM versus upstream scenarios. Reproduce uncertain upstream behavior with small temporary probes.
4. **Review independently.** Request correctness and idiomatic Rust review, including ownership, error behavior, lifetime cleanup, bounds, and whether the oracle actually exercises the feature.
5. **Fix with evidence.** Route a finding to its source owner, add a focused regression, and rerun the failing case first. Never weaken the oracle to match the implementation.
6. **Verify the stable result.** Run the aggregate checks once the source is coherent. Repeat after meaningful changes, failures, or unresolved concerns, rather than running full suites continuously.
7. **Hand off clearly.** Report the implemented scope, exact check results, material native boundaries, documentation, and any remaining issue.

Give concise progress updates during long work, normally within a minute. Report concrete findings, milestones, uncertainties, and next checks rather than promises or elapsed activity.

## Implementation conventions

- Compose by domain with focused responsibility files and thin domain entrypoints. Prefer small editable modules over a large implementation file or a speculative framework.
- Do not add handwritten code comments, including doc comments. Put rationale and examples in Markdown.
- Keep tests outside `src` in the mirrored `crates/rstate/tests` tree. Cargo uses `autotests = false` and the explicit `tests/main.rs` target; wire children through domain test modules.
- Test observable behavior, not every helper. Do not expose private internals solely to test them.
- Keep the core synchronous and single-threaded. `Rc`, `Weak`, and short `RefCell` borrows are deliberate; do not replace state ownership with `Arc`/locks or add an async runtime without a real requirement. Shared error ownership can still use `Arc` to preserve error interoperability.
- Release interior borrows before invoking reducers, projectors, comparators, subscribers, inspectors, codecs, storage, error callbacks, or user formatting code. Consider captured-value destruction during cleanup too.
- Keep context, event, and selected-value `Clone`, `Debug`, serialization, `Send`, and `Sync` bounds local to operations that actually need them. Manual `Clone` and opaque/conditional `Debug` avoid accidental generic bounds.
- Use `Fn` where callbacks can reenter and `FnOnce` for consumed effects. Recoverable callback failures use typed `Result` errors; panics remain programming faults.
- Preserve `StoreError` interoperability with `Send + Sync` error containers and direct source chains. `thiserror` is the mandatory error dependency; `anyhow` is appropriate only if executable tooling needs it.
- Serialization remains optional through the `json` feature. Do not impose `Serialize` on the state-management core.
- Respect strict Clippy settings in `Cargo.toml` and `clippy.toml`. Do not relax policy to finish a feature; use only narrow, justified exceptions when necessary.

For example, `src/store/weak.rs` is covered by `tests/store/weak.rs`. The browser adapter and its tests have their own focused trees. Context history relies on immutable replacement; arbitrary interior mutation cannot be reversed.

## Upstream reference and parity

The selected reference is [`@xstate/store` 4.3.0](https://github.com/statelyai/xstate/tree/9b3a88fa43f4be5433797acfba18f27c40494bd2/packages/xstate-store), immutable commit `9b3a88fa43f4be5433797acfba18f27c40494bd2`.

Read official implementation, exports, tests, and migration notes together. README prose and type declarations alone were insufficient: the validation implementation ignores successful schema output values, which an early transformation assumption would have missed. Verify current `main` when researching, but do not silently move the test pin.

`test-app/reference.lock.json` records source checksums and the license. `npm run reference` verifies or reconstructs the ignored source cache. Add any newly needed upstream modules and aliases through that manifest and fetch workflow. Temporary `/tmp` checkouts are useful research scratch space, not durable prerequisites.

The oracle must execute actual pinned upstream code. The Rust adapter must call the actual core API, not reproduce its behavior in JavaScript or manually manufacture feature traces. Keep fixture adapters small; they are not a general JavaScript SDK.

Compare ordered traces, not only final JSON:

- Context/snapshot and selected-value identity relationships, tracked from retained Rust handles and JavaScript references.
- Projector counts, lazy reads, equality suppression, callback entry/return timing and ordering.
- Effects, emissions, inspection, storage calls, recoverable failures, and whether state was already committed.
- Lifecycle: last unsubscribe, reactivation, dropped handles, and nested sends.

Use explicit equivalent typed fixture values. Rust event decoding can discard raw JavaScript spelling or default missing fields. Standard Schema declarations also determine upstream known event names: declare pass-through schemas for supported variants and leave genuinely unknown variants undeclared.

Normalize platform/protocol metadata only where documented. Test intentional native differences separately instead of hiding them in comparison. JavaScript trigger proxies and heterogeneous selector dictionaries can use ordinary Rust event constructors and typed structs.

See [conformance/README.md](conformance/README.md) for scenario contracts and [test-app/README.md](test-app/README.md) for adapter boundaries.

## Behavioral traps worth retaining

| Area | Regression to protect |
| --- | --- |
| Dispatch | Internal `enqueue.trigger` events run FIFO in one macrostep. Callback `store.send` reenters synchronously; an outer mailbox would change behavior. |
| Commit order | State/history commit, ordinary and selected subscribers, inspection, persistence, then user effects/emissions. Preserve each frame's captured snapshot during nested dispatch. |
| Errors | Subscriber errors drain remaining notifications, retain committed state, and skip later stages. Inspector/emitted-listener/effect errors fail fast. |
| Effect scopes | Reads stay on the dispatch snapshot throughout synchronous effects, becoming live only after the whole loop succeeds. Failed loops leave retained scopes pinned. |
| Ownership | Ordinary subscription handles do not cancel on drop. Scoped guards do. Weak captures avoid user-created owning callback cycles; public selections and effect scopes intentionally own stores. |
| Derivation | Reads reserve dependency order without creating live observation. Last unwatch, re-read, equal cached identities, and unobserved child dependencies all affect reactivation and notification order. |
| Combined state | Same-root diamonds notify coherently once; duplicate and ancestor/derived inputs need tests. Separate independent-store sends remain separate commits. Internal readers may retain an expired input's last materialized value after public selection handles disappear, without cross-store owning cycles. |
| Inspection | Registration reports current state immediately and follows live listener insertion/removal. An initial callback error removes its own registration before returning, releasing captures outside interior borrows and retaining nested commits. This native policy intentionally differs from upstream; see `API_STABILITY.md`. |
| Snapshot history | No-op domain events consume capacity, reset is undoable, and undo/redo do not replay effects. Configured reset defaults differ from hydrated visible initial state. |
| Persistence | Guard stale outer writes with commit revisions and clear epochs. Auto-save failures must not fail a committed domain action. Explicit save must report queued failures or `Superseded` instead of false success. Active synchronous I/O rejects reentrant explicit save/clear. |
| Validation | Metadata alone is inert. Checks do not transform values; internal-trigger event payloads bypass event checks. Validate final context/emissions before commit, and map only validation errors to `can == false`. Native defaults, hydration, and control safety checks are explicit composition policies. |

These were real sources of tests or fixes in this session. Consult the README and existing tests for exact API behavior before changing them.

## Verification

Run these from the repository root after the source stabilizes. The formatting, Clippy, and all-feature test commands match [.github/workflows/rust.yml](.github/workflows/rust.yml); default-feature and MSRV checks were additional session gates.

```sh
cargo fmt --all -- --check
cargo test --locked --workspace --all-features
cargo test --locked -p rstate --no-default-features
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo clippy --locked -p rstate-wasm --target wasm32-unknown-unknown --all-features -- -D warnings
cargo +1.85.0 check --locked --workspace --all-features
git diff --check
```

The repository tracks stable Rust with rustfmt, Clippy, and `wasm32-unknown-unknown` in `rust-toolchain.toml`. Rust 1.98.0 was the stable compiler used during this session; the separate minimum-version check used 1.85.0. Install that toolchain if the MSRV command needs it; do not change the manifest baseline merely to make a check pass.

The browser workflow needs `wasm-pack`, Node 24 or newer, npm 11, and Playwright Chromium. Follow the test-app README for its verified tool installation commands and `package.json` for current package requirements.

```sh
cd test-app
npm ci
npx playwright install chromium
npm run build
npm run test:prepared
```

`build` verifies the pinned reference, rebuilds the real WASM adapter, type-checks, and builds Vite. `test:prepared` intentionally skips preparation; use it only after fresh engines. Alternatively, `npm test` prepares engines itself. For targeted iteration, run the relevant native/browser case first, then return to the aggregate gates.

For interactive testing, `npm run dev` prepares engines and starts Vite, normally at `http://127.0.0.1:4173`. Confirm the printed URL and server state. Do not assume a previous session's server, process handle, generated bindings, browser installation, or temporary checkout survived.

For documentation-only changes, check the actual diff, local links, commands, and whitespace; do not rerun unrelated code suites just to update Markdown.

## Verified checkpoint

At commit `6a59c23`, the final recorded verification was **131 all-feature native tests, 130 without default features, and 48 browser tests**, plus formatting, strict host/WASM Clippy, Rust 1.85, and fresh WASM/TypeScript/production builds. Independent review closed with no remaining actionable findings. These are historical counts; discover and report the current results next session.

Implemented scope is listed in `AGENTS.md` and the README: the event-driven store core, identity-aware snapshots, effects/emissions, factories, inspection, explicit derived selections, weak/scoped ownership, reset/history, synchronous snapshot persistence, and typed opt-in validation.

Still deferred: general atoms, automatic dependency tracking, async runtimes/storage/validators, arbitrary extension frameworks, event-replay history, automatic validation transforms, general schema languages, and actor integration. Persistence also defers event logs, migrations, partial merges, throttling, and broadcast. Revisit scope through concrete use cases and upstream research, not an automatic checklist.

| Commit | Milestone |
| --- | --- |
| `6a59c23` | Typed schemas and opt-in synchronous validation |
| `a4ef4ab` | Reset, snapshot undo and synchronous persistence |
| `66633eb` | Weak handles, scoped subscriptions and combined selections |
| `44666cc` | Borrowed-event transition inspection |
| `6f16b20` | Strict Cargo and Clippy policy |
| `ebfc75f` | Lazy selection composition |
| `4490cea` | Input-initialized store factories |
| `11e17e8` | Error interoperability and public Debug support |
| `d423c69` | Initial core and test foundation |

## Finish and commit

The user requested commits separately after verified increments. Commit when requested; do not automatically push, amend, reset, or include other work.

Inspect status and the full task diff, check whitespace, stage explicit task paths or hunks, inspect the staged result, commit with a concrete behavior-oriented title, then verify status and report the hash. During selection work, concurrent lint changes were preserved and staged separately; mixed-file commits were checked in an isolated staged snapshot. Use that technique when necessary rather than overwriting another contributor's work.

Report the actual worktree state. If unrelated changes remain, say so; do not call the tree clean. Keep generated caches, bindings, build outputs, temporary probes, credentials, and local server artifacts out of commits.

## Suggested continuation prompt

“Read AGENTS.md and PLAYBOOK.md, inspect the current repository state, and retrieve focused approved knowledge. For the next requested feature, research the pinned upstream behavior, propose the smallest clean Rust API and acceptance tests, then orchestrate implementation, real parity tests, independent review, and the final checks. Preserve current scope and other contributors' edits. Do not commit until asked.”
