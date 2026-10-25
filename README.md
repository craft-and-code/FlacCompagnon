# Website source

`site/` contains the homepage template, styles, scripts and visual assets. It
is not a complete static site by itself: the documentation pages are generated
from `docs/` and `docs/fr/` into `dist-site/`.

Do not open `site/index.html` with `file://`. Build it and use the local web
server instead:

```sh
npm ci
npm run build:site
npm run preview:site
```

Then open <http://localhost:4174/> for French or
<http://localhost:4174/en/> for English. The preview server serves the
generated `dist-site/` directory, including the two localized landing pages,
the bilingual Docs navigation and every analysis guide.

`npm run build:site` also validates every generated local link, fragment,
language counterpart, SEO metadata and documented CLI option. `dist-site/` is
a generated, ignored directory; edit the sources in `site/`, `docs/`,
`docs/fr/` and `scripts/site/` instead.

## Local JSON report reader

Open <http://localhost:4174/report.html> (French) or
<http://localhost:4174/en/report.html> (English), also linked from the homepage.
Drop one FlacCompagnon JSON report into the large drop zone, or activate the
zone with a mouse or keyboard to choose a local file. An album keeps its track
order and displays the selected track's measurements. Clear removes the report
from memory. A rejected replacement leaves the previous report intact.

The reader supports the version-1 envelope, legacy album reports and individual
track reports, with Phémios's validation limits (16 MiB and 2,000 tracks).
Reading uses the browser's File API. No report, audio file, path or result is
uploaded, persisted in browser storage or used to trigger a new audio analysis.
The dedicated page loads only self-hosted static assets and sets
`connect-src 'none'` and `form-action 'none'`; it has no analytics or API calls.

`site/vendor/phemios-analysis/` contains the renderer exported from Phémios and
used by its Storybook stories, with provenance and licenses. It includes its
runtime; the website does not need Storybook, a CDN or a sibling checkout to
build. Refresh instructions are in that directory's README.

Run `node --test tests/site-report.test.mjs` for file admission, invalid-report
preservation, stale read cancellation, parser limits and provenance checks.
Browser checks: import a real album; select another track; expand rubrics;
try an invalid JSON without losing the current report; clear; verify both
languages. Drop and picker selection share the same local admission controller.
