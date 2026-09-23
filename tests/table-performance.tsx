// Browser regression fixture using the real search control and results table.
// Build with build-table-performance.mjs; no Tauri backend or audio is needed.
import { useLayoutEffect, useMemo, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { TopBar } from "../src/components/TopBar";
import { ResultsTable } from "../src/components/ResultsTable";
import type { TableScrollHandle } from "../src/components/useTableWindow";
import { nextSort, sortFiles, type SortState } from "../src/components/tableSort";
import { fileSearchFields, matchesSearch } from "../src/format";
import type { FileAnalysis } from "../src/types";
import "../src/theme.css";
import "../src/shared.css";
import "../src/App.css";

const noop = () => {};
const count = Number(new URLSearchParams(location.search).get("count")) || 2000;
const initialFiles: FileAnalysis[] = Array.from({ length: count }, (_, i) => ({
  path: `/fixture/track-${i}.flac`,
  file_name: `${String(i).padStart(5, "0")} ${i % 100 === 0 ? "Zakk Wylde" : "Other Artist"}.flac`,
  format: "FLAC", codec: null, ext_mismatch: false, sample_rate: 44100,
  channels: 2, declared_bits: 16, duration_secs: 180, size_bytes: 20_000_000,
  bitrate_kbps: 889, modified_unix: null,
  detections: { upscaling: false, upsampling: false, transcoding: false, detail: "", summary: "Clean" },
  cutoff_hz: 20000, cutoff_ratio: 0.9, real_bit_depth: 16, lattice_score: null,
  fake_stereo: false, badge: null,
  clipping: { clipped: false, clipped_samples: 0, clip_events: 0, peak: 0.9,
    peak_dbfs: -1, true_peak: 0.9, true_peak_dbtp: -1 },
  dr_db: 12, flac_md5: { state: "Match" }, file_md5: null, file_crc32: null,
  error: i === count - 1 ? "Fixture decoding error" : null,
}));

function Fixture() {
  const [files, setFiles] = useState(initialFiles);
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<SortState | null>(null);
  const [selected, setSelected] = useState<string[]>([]);
  const [editing, setEditing] = useState<string | null>(null);
  const [metrics, setMetrics] = useState("Ready");
  const start = useRef(performance.now());
  const tableRef = useRef<TableScrollHandle>(null);
  const empty = useMemo(() => new Map(), []);
  const missing = useMemo(() => new Set<string>(), []);
  const visible = useMemo(() => query.trim()
    ? new Set(files.filter(f => matchesSearch(fileSearchFields(f), query)).map(f => f.path))
    : null, [files, query]);
  const displayed = useMemo(() => {
    const matching = visible ? files.filter(f => visible.has(f.path)) : files;
    return sort ? sortFiles(matching, sort) : matching;
  }, [files, sort, visible]);
  useLayoutEffect(() => {
    let second = 0;
    const first = requestAnimationFrame(() => {
      second = requestAnimationFrame(() => {
        const rows = [...document.querySelectorAll<HTMLTableRowElement>("tr[data-path]")];
        const heights = rows.filter(r => !r.hidden).map(r => r.getBoundingClientRect().height);
        setMetrics(`${Math.round(performance.now() - start.current)} ms to paint; ` +
          `${rows.length} mounted rows; height ${heights.length ? `${Math.min(...heights)}–${Math.max(...heights)} px` : "n/a"}`);
      });
    });
    return () => { cancelAnimationFrame(first); cancelAnimationFrame(second); };
  }, [query, files, sort]);
  return <div id="app" onPointerDownCapture={() => { start.current = performance.now(); }}
    onInputCapture={() => { start.current = performance.now(); }}>
    <TopBar busy={false} hasReport canGenerateSpectrograms={false} ffmpegAvailable={false}
      onSearchChange={setQuery} selectedCount={selected.length} onSelectAll={() => setSelected(files.map(f => f.path))}
      onDeselectAll={() => setSelected([])} onInvertSelection={noop} renumberBusy={false}
      onRenumberTracks={noop} onPick={noop} onSave={noop} onExportPlaylist={noop}
      onGenerateSpectrograms={noop} onReset={noop} onOpenConvert={noop}
      onRefreshPresence={noop} checkingPresence={false} missingCount={0}
      onRelocateMissing={noop} canUpdateReport={false} onUpdateReport={noop} />
    <p>{files.length} files; {visible?.size ?? files.length} matches; selected {selected.length}. {metrics}</p>
    <div><button onClick={() => tableRef.current?.scrollToPath(files[files.length - 1].path)}>Jump to last row</button>
      <button onClick={() => tableRef.current?.scrollToPath(files[0].path)}>Jump to first row</button></div>
    <ResultsTable files={files} tags={empty} covers={empty} missing={missing} filterKey={query} scrollRef={tableRef}
      nowPlaying={null} selectedPaths={selected} displayedFiles={displayed} sort={sort}
      onSortChange={column => setSort(current => nextSort(current, column))}
      editingPath={editing} renameBusy={false} onSelectRow={path => setSelected([path])}
      onStartRename={setEditing} onCancelRename={() => setEditing(null)}
      onSubmitRename={(path, name) => {
        setFiles(current => current.map(f => f.path === path ? { ...f, file_name: `${name}.flac` } : f));
        setEditing(null);
      }} onReorder={paths => {
        const byPath = new Map(files.map(f => [f.path, f]));
        setFiles(paths.map(path => byPath.get(path)!));
      }} onReveal={noop} onTogglePlay={noop}
      onDelete={path => setFiles(current => current.filter(f => f.path !== path))} />
  </div>;
}
createRoot(document.getElementById("root")!).render(<Fixture />);
