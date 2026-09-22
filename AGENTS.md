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

## Factor out shared behaviour, not just shared lines

**The second time the same behaviour appears, it becomes one component, one
hook, or one function — used by both call sites.** This applies across the
whole repo (React components, hooks, `format.ts` helpers, `shared.css` rules,
Rust modules) and it is not primarily about file length or typing less. It is
about correctness: two copies of the same behaviour are two things that can
disagree, and they will, because a fix applied to one is a fix the other never
receives.

This rule was written after a real bug. Three drop targets — the cover box,
the results list, the conversion panel — each hit-tested incoming drop
positions with their own copy of the same rectangle arithmetic. The shared
coordinate conversion underneath them was wrong, but the two left-hand,
wide targets absorbed the error and kept working, so only the right-hand one
appeared broken. Several rounds of debugging went into the one panel that
looked at fault, because the duplication hid that the defect was common to all
three. One `dropZones.ts` later, there is a single declaration site, a single
lookup, and no arithmetic at all. **Duplication doesn't just cost maintenance;
it hides where a bug actually lives.**

Practical consequences:

- Two components needing the same visual treatment: the rule goes in
  `src/shared.css` (`.link-btn`, `.btn`, `.icon-btn`), not copied into each
  component's stylesheet.
- Two components needing the same markup and interaction: extract a component
  (`IconButton`, `MarqueeText`) and pass the differences as props.
- Two call sites needing the same pure logic: extract a function into
  `src/format.ts` or the appropriate Rust module (`isAudioPath`,
  `convert::pcm`).
- Two places needing the same stateful behaviour: extract a hook
  (`useLatest`, `useColumnPrefs`).
- The parameters that differ become arguments. If that argument list grows
  past what one sentence can describe, the two cases were genuinely different
  after all — that's the signal to stop, not to add a sixth boolean flag.

What this rule is *not*: an instruction to unify things that merely look
alike today. Two functions with identical bodies and unrelated reasons to
change are better left apart — coupling them means a change to one silently
alters the other. The test is whether they would have to change *together*.

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

### Styling

- **One component, one CSS file.** `ComponentName.tsx` owns
  `ComponentName.css`, imported as a side effect at the top of the component
  (`import "./ComponentName.css";`) — the same "one module, one
  responsibility" rule as the rest of this file, applied to styles: a
  component's look lives next to the markup that uses it, not in a shared
  monolith where an edit to one component's styles risks another's.
- **Rules used by more than one component go in `src/shared.css`** (buttons,
  status colours, the spinner, the `.modal-card` shell — anything that would
  otherwise be copy-pasted and drift out of sync between components).
  `src/theme.css` holds the CSS custom properties (light/dark palettes) and
  the base reset; both are imported once, from `main.tsx`, ahead of every
  component's own stylesheet.
- Before adding a new rule, check `shared.css` first — a class only one
  component currently uses but that conceptually belongs to the design system
  (another button variant, another status colour) belongs there, not
  re-declared locally.
- CSS classes are global (no CSS Modules), same as before the split: nothing
  stops one component's file from defining a selector another component's
  markup also matches. Name classes after the component that owns them
  (`tag-cover-*`, `lookup-*`) to keep collisions unlikely, and do not rely on
  import order for specificity.
- **Corner radius comes from the `--radius-*` scale in `theme.css`**
  (`--radius-lg`/`card`/`md`/`sm`/`xs`), not a hand-picked number — a
  one-off value is how a panel, a button and an input each end up with a
  subtly different curve nobody chose on purpose. Pick the tier by role, not
  by component: `--radius-lg` for a major content-area container
  (interchangeable siblings like the tag panel, dropzone, results table and
  progress bar shell must use the *same* one, since one replaces another in
  the same layout slot), `--radius-card` for dialogs/pop-ins, `--radius-md`
  for anything clickable or typeable (buttons, text inputs, selects, rows
  inside a dialog), `--radius-sm` for small in-row controls (icon buttons,
  tags), `--radius-xs` for tiny inline elements. A pill (`border-radius:
  50%` of the height) or a circle are a different shape language, not a
  missing tier — leave those as their own literal value.
- A nested rounded element should almost never reuse its container's exact
  radius unless it sits flush against it with no gap (an overlay with
  `inset: 0`, an image filling its frame edge-to-edge) — then it must match
  exactly, or the two curves visibly disagree at the edge. Otherwise it
  should be a smaller tier. The precise relationship, for when it matters
  enough to compute rather than just picking the next tier down: `inner
  radius = outer radius − the gap between them`, clamped to `0` once the gap
  is bigger than the outer radius. (Source: [How to calculate the
  border-radius of nested
  elements](https://douglasmoura.dev/how-to-calculate-the-border-radius-of-nested-elements).)
- **Colour comes from the `--*` custom properties in `theme.css`, never a
  hand-picked hex/rgb.** Status colours especially: `--ok`/`--bad`/`--warn`/
  `--mid` (and the per-detection `--clean`/`--upscaled`/`--upsampled`/
  `--transcoded`) each mean one specific thing across the whole app — reusing
  `--bad` for a *different* kind of warning, or introducing a fresh red
  because `--bad` "felt slightly off" in that one spot, breaks the mapping a
  user has already learned from the rest of the app. Both palettes (light/
  dark) live behind the same variable name specifically so a component never
  has to think about which one is active.
- **A control's size matches the rendered box of whatever it sits next to,
  not an arbitrary number.** The default icon button is 20×20 (`.icon-btn`,
  IconButton.css) — the size to reach for first. When a control genuinely
  needs to be bigger (it stands alone rather than repeating down a list, or
  its glyph is too small to read at 20px), size it to match a specific
  neighbour's *rendered* box — e.g. `.topbar-toolbtn` is 32×32 because that's
  `.topbar-search`'s own input box (13px text + 7px padding + 1px border each
  side), not a round number picked by eye. Wrapping same-height controls into
  a bordered pill (`.topbar-selection-actions`, `.ext-plusminus`) adds that
  pill's own border on top of each child's box — shrink the children by the
  border width so the group doesn't end up reading taller than its
  neighbours.
- **An icon centred on a colour-filled background must land exactly on both
  axes**, not just close. Flex centering (`align-items`/`justify-content:
  center`, the default via `.icon-btn`) splits the box size minus the icon
  size into two margins; when that remainder is odd (a 28px circle with a
  13px icon splits 15px into 7.5px each side), the two margins can't both
  round to the same whole pixel and the icon reads as shifted toward one
  corner — invisible on a transparent background, obvious on a filled or
  coloured one, where there's nothing else to draw the eye away from the
  asymmetry. Either keep the box and icon sizes the same parity (both even,
  so the split is a whole number), or centre with `position: absolute; top:
  50%; left: 50%; transform: translate(-50%, -50%)` instead, which centres on
  the icon's actual — possibly fractional — size rather than a margin that
  has to round. See `.icon-btn.playback-transport-playpause` in
  PlaybackTransport.css for the pattern.

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

## Working from other people's code

An AI assistant working on this repo has read a huge amount of public code
during training, and can fetch and read live repositories or web pages when
browsing is available. Left unguided, that's a real risk for a project whose
whole premise — detecting fake-lossless audio via spectral heuristics —
overlaps with existing open-source and freeware tools (spectrogram viewers,
fake-detection utilities, and the like). Asked to "add a heuristic for X," an
assistant can end up reproducing a known tool's approach, naming, or even its
bugs without ever being told to copy anything — it just resembles what it has
already seen. (Source: [Coder avec l'IA sans pomper le projet d'un
autre ?](https://korben.info/ia-clone-projet-open-source.html) — a shipped app
pulled after it turned out to share its name, its features, and even its bugs
with an existing open-source project, entirely unintentionally.)

- **Do not fetch, clone, or read another project's source** (open-source or
  not) to use as a model for how to implement something here, unless the
  maintainer explicitly asks and scopes it. Reading a *format specification*
  or *protocol documentation* is fair game — that's how a decoder gets written
  correctly — reading another project's *implementation* to copy its structure
  is not.
- Implement from the spec and from first principles, not from memory of a
  specific known codebase. Describing an algorithm in your own words and
  writing it fresh is fine; reproducing a particular project's actual code —
  variable names, structure, comments, bugs and all — is not, even unprompted
  and even recalled from training data rather than fetched live.
- Before naming anything new that ships to users (a new binary, a new
  feature), check the name isn't already in use elsewhere first.
- Before a release, a deliberate comparison pass against the handful of
  existing tools this app's feature set overlaps with is worth the same
  checklist status as `cargo clippy`/`cargo test` — not to avoid *similar
  features* (that's normal and expected of any tool in the same space) but to
  catch anything that's copied rather than reimplemented: matching interface
  text, matching internal naming, matching edge-case bugs.

## Behaviour to preserve

Changes to the frontend must not break these, which are easy to lose in a
refactor because they live outside the normal React tree:

- **Native OS file drop** (`onDragDropEvent`) — dropping audio files, folders,
  a saved `.json` report, or an image onto the cover box, or audio files/
  folders onto the conversion panel's own drop zone (`useNativeDrop`'s
  `convertPanelRef`/`overConvert`, routed to `ConvertPanel` rather than the
  results table).
- **Native menu bar** (built in `src-tauri/src/menu.rs`) — emits `menu://action`
  events the frontend routes to the same functions the toolbar buttons call.
  On macOS the app menu must keep its Hide/Hide Others/Show All items, or
  Cmd+H silently stops working (there is no item for the shortcut to bind to).
- **Playback lives in Rust**, so it survives a webview reload; the frontend
  stops it unconditionally at startup.
- **The window starts hidden** and is revealed on `load` to avoid a flash of
  unstyled content.
- **Display order** (manual drag reorder) is what CSV/JSON/M3U exports use.
