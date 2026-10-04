# Frontend regressions

Run `npm test` for the complete JavaScript regression suite (frontend and site).
It includes search, table windowing, analysis cells, detection labels, stereo
labels and asynchronous state. `npm run test:search` and `npm run test:table`
remain available for focused checks.

Native menu and keyboard routing tests verify that the toolbar's unavailable
state also prevents Reset, exports and table shortcuts during backend tasks,
while preserving keyboard ownership for text fields and modal dialogs.

The asynchronous state tests control the completion order of backend reads.
They cover stale tag/cover responses after save, reset or removal; completed
saves that must not refetch removed rows; initial prefetch after import; overlapping
filesystem checks; delayed playback/lookup responses; edits made during a save;
cover drops across selection changes; storage failures; and duplicate column
preferences. They use the production state controllers without
an audio device, network access or a Tauri runtime.

Run `node tests/benchmark-row-reorder.mjs` from the repository root to compare
the pre-audit row reorder logic with the production helpers. The synthetic
case has 10,000 paths, 5,000 selected paths, one warmup and ten measured
iterations. It checks that both versions produce the same order and reports
their mean durations. One local run measured 201.8 ms before and 0.79 ms after
replacing repeated array membership scans with sets. These numbers describe
the reorder logic alone; they exclude React rendering, native drag events,
audio reads and IPC, and do not predict desktop interaction latency.

For browser performance and interaction checks, run
`node tests/build-table-performance.mjs`, then serve the printed temporary
directory with a local HTTP server. Open `/?count=10000` on that server.
This fixture bundles the actual production search control and results table,
using synthetic files without reading audio or invoking the Tauri backend.

- Search for `Zakk Wylde`, then clear with the cross. All 10,000 results must
  return; only the viewport plus a small buffer should have mounted rows.
  The timing includes input/pointer handling and two animation frames, so it
  is an approximate interaction-to-paint measurement, not a CPU benchmark.
- Jump to the last row, search for a nonexistent name, then clear. The table
  must return to the top with real rows, without a blank scrolled region.
- Check File sorting, Select all (including offscreen files), and normal
  scrolling to the final error row.
- Click a selected filename again after half a second to rename it. Scroll
  away while editing: the editor must remain mounted with its draft intact.
- Check drag reordering in natural order and column visibility/reordering.

Native file drops and audio playback require the desktop app; the fixture
intentionally does not simulate their backend.
