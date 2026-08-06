# FlacCompagnon — working conventions

A cross-platform desktop app (Rust + Tauri v2) that detects fake "lossless" audio.
This file is the contract for how code in this repository is written. Read it
before making changes.

## Language

- **Code, comments, commit messages, UI strings: English.**
- **Conversation with the maintainer: French.**

## Architecture

```
core/                     Pure Rust analysis library. No Tauri dependency, fully unit-testable.
  src/decode/             One module per decode path (generic, FLAC+MD5, DSD, playback).
  src/dsd/                DSD container parsing + the spectral heuristics, kept apart.
  src/tags/               Tag read/write; cover art in its own file.
src-tauri/                The Tauri v2 desktop app wrapping `core`.
  src/lib.rs              Wiring only: modules, run(), generate_handler!.
  src/commands/           One file per command domain.
  src/lookup/             Online tag lookup, one file per provider.
  src/menu.rs             Native menu bar.
  src/playback.rs         Audio engine (owns the cpal stream on its own thread).
src/                      Frontend: React + TypeScript, built by Vite.
index.html                App shell only — see "index.html" below.
site/                     The GitHub Pages marketing site (unrelated to the app build).
```

The split is deliberate: anything that can be tested without a GUI belongs in
`core`, so the analysis algorithms stay verifiable with `cargo test` alone.

## Frontend rules

### One module, one responsibility

- **A component file does one thing.** If you cannot describe a file's job in a
  single sentence without "and", split it.
- **~200 lines is the target, 300 the hard ceiling** for a component file.
  Passing it means the component is doing several jobs — split it rather than
  letting it grow. (A file that is one long static list, e.g. a table of
  labels, is the exception.)
- A component owns **its own markup, its own local state, and its own event
  handlers**. Markup for a feature does not live in `index.html` while its
  logic lives in a module — that separation is by layer, not by responsibility,
  and it is exactly what this rule exists to prevent.

### State

- Local state stays in the component that uses it (`useState`).
- Shared state is lifted to the nearest common parent and passed down as props.
  No global mutable singletons.
- Server/backend state (analysis results, tags, covers) is fetched through
  `src/api.ts` and cached in the owning component, never fetched ad hoc from a
  leaf component.

### Boundaries

- **All Tauri `invoke` calls live in `src/api.ts`.** A component never calls
  `invoke` directly — it calls a typed wrapper. This keeps the backend surface
  in one reviewable place and makes the payload types match `src/types.ts`.
- **All Rust payload types are mirrored in `src/types.ts`.** When a `serde`
  struct changes on the Rust side, update `types.ts` in the same change.
- Pure formatting helpers (durations, sizes, labels) live in `src/format.ts`
  and stay free of DOM and React.

### Rendering

- Let React render. Do not reach into the DOM to patch what a re-render would
  already produce (no `innerHTML`, no `outerHTML`, no manual `classList`
  toggling for state that props already describe).
- `useRef` is for genuine DOM measurement — element geometry, hit-testing,
  focus — not for bypassing the render cycle.
- Lists get a stable `key` (the file path), never the array index.

### index.html

`index.html` is the **app shell**: `<head>`, a root mount node, and the module
script tag. Nothing else. Any markup a component could own belongs to that
component.

## Rust rules

### Layering

- `core` must not depend on Tauri, and must not do I/O beyond reading the audio
  files it is given.
- Tauri commands stay thin: validate, call into `core`, map errors to `String`.
  Real logic belongs in `core` where it can be tested.
- `src-tauri/src/lib.rs` is **wiring only** — module declarations, `run()`, and
  the `generate_handler!` list. A command's body does not live there. This is
  the Rust counterpart of the `index.html` rule above, and it exists for the
  same reason: that file grew to 750 lines of unrelated commands once, because
  "just one more command here" is always the path of least resistance.

### One module, one responsibility

- **The frontend's size rule applies here too: ~200 lines target, 300 the hard
  ceiling.** Same test — if you cannot describe a file's job in one sentence
  without "and", split it.
- **The count is code, not tests.** A `#[cfg(test)] mod tests` at the bottom
  doesn't push a file over: this file also asks for thorough tests next to the
  code they cover, and the two rules must not fight. Thorough tests are never
  the reason to split a module.
- **Exemption, and how to claim it.** A file that is *one* algorithm or *one*
  subsystem may exceed the ceiling when splitting it would only scatter an
  argument that reads best top-to-bottom (a DSP method with its derivation, a
  thread-owning engine). Claiming this is not free: the file's `//!` header must
  say **why it is one unit and what splitting it would cost**. A file over 300
  lines with no such note is drift, not a decision.
- Group by **domain**, not by layer. `commands/tags.rs` (everything about tags),
  not `structs.rs` + `handlers.rs`. When a module gains a second file, make it a
  folder with a `mod.rs` that carries the shared types and the module docs —
  `core/src/tags/` and `core/src/decode/` are the reference shape.
- The public API of a split module does not change shape: re-export from
  `mod.rs` so callers keep using `core::tags::CoverArt`, not
  `core::tags::cover::CoverArt`. A refactor that churns every call site is a
  refactor that will not get done.

### Practices

- **`cargo clippy` must be clean**, warnings included. A lint that is genuinely
  wrong gets a scoped `#[allow(...)]` *with a comment saying why* — never a
  crate-wide allow, never an unexplained one.
- **No `unwrap()`/`expect()`/panicking indexing on anything derived from a
  file's contents or from user input.** Audio files here are untrusted by
  definition — half the point of the app is that they may be malformed or
  actively lying. Parse with `get()`, `try_into()`, checked arithmetic, and
  return `AnalysisError`. `expect()` is acceptable only for an invariant the
  surrounding code has just established, and then it says so.
- Prefer borrowing to cloning in hot paths (the per-frame analysis loop, MDCT
  sweeps). Elsewhere, a `clone()` that buys clarity is fine — say so if it is
  not obvious.
- Public items get a `///` doc comment. `core` is published as rustdoc (see the
  docs workflow), so its public surface is read by people who cannot see the
  implementation.
- Numeric constants that came from measurement or a paper cite their source in
  a comment — a bare `30.0` is unreviewable, `30 dB (measured: native ≈ 3 dB,
  PCM-sourced ≈ 50 dB)` is not.

### Tests

- Every detection algorithm needs a unit test with independently derived ground
  truth (a reference encoder, a bit-exact replica, a known-good file) — never
  a test that just asserts whatever the implementation currently returns.
- Every parser needs a malformed-input test. "It rejects garbage without
  panicking" is a behaviour, and it is the one that breaks first on real files.
- A bug fixed is a test added, named after the symptom.
- Tests live in a `#[cfg(test)] mod tests` at the bottom of the file they cover,
  except cross-module ground-truth suites, which go in `core/tests/`.

## Testing and verification

Before saying a change is done:

```bash
npx tsc --noEmit     # frontend types (noUnusedLocals is on — dead code fails)
npm run build        # typecheck + Vite build
cargo test           # Rust workspace
cargo clippy         # Rust lints
```

**The AI assistant's sandbox has no Rust toolchain and no npm registry
access.** Rust changes and any change needing a fresh dependency can only be
reviewed by reading, not compiled — say so explicitly rather than implying a
change was verified, and ask the maintainer to run the commands above.

## Comments

Comments explain **why**, not what. The codebase's existing style — a short
paragraph above a non-obvious function explaining the constraint that shaped it
(a Tauri quirk, a race, a format limitation) — is the standard to match. Do not
add comments that restate the code.

## Behaviour to preserve

Changes to the frontend must not break these, which are easy to lose in a
refactor because they live outside the normal React tree:

- **Native OS file drop** (`onDragDropEvent`) — dropping audio files, folders,
  a saved `.json` report, or an image onto the cover box.
- **Native menu bar** (built in `src-tauri/src/menu.rs`) — emits `menu://action`
  events the frontend routes to the same functions the toolbar buttons call.
  On macOS the app menu must keep its Hide/Hide Others/Show All items, or
  Cmd+H silently stops working (there is no item for the shortcut to bind to).
- **Playback lives in Rust**, so it survives a webview reload; the frontend
  stops it unconditionally at startup.
- **The window starts hidden** and is revealed on `load` to avoid a flash of
  unstyled content.
- **Display order** (manual drag reorder) is what CSV/JSON/M3U exports use.
