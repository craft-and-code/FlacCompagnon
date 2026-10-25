# Phémios analysis report

This is the self-contained export of the exact renderer used in Phémios's
Analysis view and `Data/AnalysisReport` Storybook stories. The Solid runtime and
Lucide glyphs are included. Storybook itself is not required by the website.

The immutable source URL, revision and bundle SHA-256 are in `provenance.json`.
The corresponding source is distributed alongside the bundle in
[source.tar.gz](source.tar.gz), with the configuration and dependency lockfile
needed to run `npm ci` followed by `npm run analysis:build` after extraction.
The immutable upstream `source` permalink is also recorded in
[provenance.json](provenance.json).
The shared icon sources are in `src/components/Icon.tsx` and `icon-symbols.ts`
at the same revision. Phémios code and the generated translation subset are
MPL-2.0 (`LICENSE`). Included runtime/icon licenses are retained in
`THIRD-PARTY-LICENSES.txt`.

To refresh from a clean Phémios checkout with its dependencies installed:

```sh
node scripts/update-analysis-viewer.mjs /path/to/phemios
```

Run that command from the FlacCompagnon repository. It writes only here, bundles
the existing source without modifying Phémios, and extracts only the English
translations used by the report. Ordinary site builds use these committed
assets and do not need the sibling checkout or a CDN.

The host uses the component's validator and atomic update API. It supplies the
local File input/drop zone, localized labels and website colours. It does not
send report contents, open audio paths, store reports or run audio analyses.
