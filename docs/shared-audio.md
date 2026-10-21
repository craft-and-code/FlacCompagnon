# Shared minimal preview playback

The preview player uses the catalogue-independent `aede-audio` crate maintained in Aède, which reuses `aede-dsp` for required rate/channel adaptation. It has no Aède account, server, catalogue, listening history, normalization, EQ, crossfade or bit-perfect mode. The analysis decoder and original-integer measurements remain independent of device playback.

`core::decode::PcmStreamDecoder` remains a compatibility adapter over the shared progressive decoder. Ordinary analysis/CLI use enables decoding only, without the device or DSP dependencies. The whole-file PCM helper remains for explicit conversion/analysis workflows; preview playback never uses it.

The Tauri module is now a thin command/event adapter. A dedicated control thread owns the shared minimal player and emits throttled request-qualified position, level, natural completion and error events outside the audio callback. The decoder feeds a fixed half-second PCM ring, plus bounded packets/resampler blocks; no whole-track PCM cache or linear whole-file resampling fallback remains in playback. Natural decode EOF and device errors are distinct. Stop retires pending starts, and request identities suppress stale events. Stop commands retain their own generation, so a preempted sender cannot stop a later Play. Pause/resume/seek carry the exact displayed request ID from TypeScript through IPC and are queued before the blocking wait; a deferred control or error cannot affect a successor track.

Pause retains the stream without consuming samples; mute preserves volume. Explicit seek progressively discards the source prefix and preserves pause/gain. It replaces the stream rather than claiming instant/gapless seeking. Captured playback admission is checked between open/seek preparation stages and before replacement/resume; Stop or a newer Play prevents the prepared visit from being published. An already running filesystem/backend call may finish before cancellation is observed. Stop/seek waits for its cancelled decoder worker to retire, including a regular-file read already in progress; there is no promise to forcibly interrupt OS filesystem I/O. The callback performs no locking, allocation, logging or Tauri event emission.

## Development and publication

The workspace declares `aede-audio` once, pinned to public revision `d8c0f1a8c134b7366da0983367a9f063527085b7` of `https://github.com/craft-and-code/aede`. Core decoding, the native preview adapter and its test harness inherit that declaration with their required features. Cargo resolves `aede-dsp` from the same Git revision, and the lockfile records both identities. No adjacent Aède checkout or local Cargo patch is needed. The technical source tag `audio-core-d8c0f1a8` makes the commit retrievable without publishing App installers; the dependency uses the full SHA rather than the tag.

Before updating this pin, publish the reviewed Aède source, update the workspace declaration and lockfile, and verify an independent checkout with `cargo metadata --locked` plus the relevant library tests. Do not copy the shared implementation into this repository. Dependency resolution alone does not certify native or cross-platform execution.

## Verification without the App

`cargo test -p flaccompagnon-core --locked --offline` includes the `native_preview` harness, which compiles the actual thin Tauri adapter against an inert event sink and runs its command-generation/input guards. Aède's `cargo test -p aede-audio --all-features --locked --offline` exercises the shared decoder, queue, format adaptation, callback consumption and cancellation with real temporary WAV fixtures and no device. App compilation, native event delivery and physical listening remain separate acceptance checks.
