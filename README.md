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
