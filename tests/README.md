# Frontend regressions

Run `npm run test:search` and `npm run test:table` for the pure logic checks.

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
