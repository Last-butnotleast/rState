# Project scope

Implement a small synchronous Rust state-management library with behavioral compatibility for the selected core of @xstate/store 4.3.0, pinned to commit 9b3a88fa43f4be5433797acfba18f27c40494bd2. The browser test app must exercise the actual Rust library through its separate WASM adapter.

Keep typed events, shared immutable snapshots, pure transitions, availability checks, synchronous dispatch, subscriptions, effects, emissions, own-context selections with explicit one-source mapping and reusable input-initialized store factories. General atoms, tracked cross-store selectors, async runtimes, extensions, persistence, schemas, inspection and actor integration are outside the current scope.

# Code organization

- Organize implementation by domain, with focused responsibility files and domain entrypoints that compose their children.
- Do not add handwritten code comments, including documentation comments. Put usage explanations and design rationale in Markdown documentation.
- Keep tests outside implementation directories in a separate mirrored domain tree. Do not add inline test modules.
- Preserve other contributors' edits and coordinate changes to public contracts.
- Keep the core independent of JavaScript, serialization and async runtimes. Do not require contexts to implement Clone, Serialize, Send or Sync.
- Release interior borrows before invoking user callbacks. Preserve synchronous nested dispatch and shared notification scheduling.

# Engineering knowledge

Before substantive engineering planning, implementation or review, use the installed cognee:memory skill to search the software_engineering dataset, ID 39afdfe2-e6f3-500a-ae40-b13833cf99ff. The installed helper at ~/.agents/skills/cognee-memory/scripts/cognee.sh discovers the current plugin and searches with --graph and the explicit dataset ID. Search focused coding-style, architecture and package questions; do not load unrelated datasets.

Approved preferences are scoped evidence, separate from sourced technical references. Current user instructions and repository evidence take precedence. A relevant fresh parent brief may avoid duplicate recall. If recall is unavailable, state that limitation and proceed without inventing remembered rules.

Knowledge retrieval is read-only. Keep automatic capture disabled. Do not upload sessions, sync, improve, cognify, publish knowledge, create datasets or change datasets without explicit authorization.

The primary agent delegates substantial implementation to coding agents and requests an independent review when ready. Agents do not recursively delegate unless the parent explicitly asks.

# Verification

Run appropriate native tests, rustfmt and Clippy for Rust changes. Browser-facing changes must also pass type checking, production build and the relevant browser parity tests. Test observable behavior, identity, ordering, errors and reentrancy rather than private implementation structure.
