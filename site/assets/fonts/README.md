# Self-hosted web fonts

These `.woff2` files are committed on purpose: the site loads them from here, so
it makes **no third-party request** (no Google Fonts call, nothing about the
visitor leaves this domain) and both the build and a local checkout work fully
offline.

| File | Family | Weight |
| --- | --- | --- |
| `space-grotesk-400.woff2` | Space Grotesk | 400 |
| `space-grotesk-500.woff2` | Space Grotesk | 500 |
| `space-grotesk-600.woff2` | Space Grotesk | 600 |
| `space-grotesk-700.woff2` | Space Grotesk | 700 |
| `ibm-plex-mono-400.woff2` | IBM Plex Mono | 400 |
| `ibm-plex-mono-500.woff2` | IBM Plex Mono | 500 |

**Subset:** latin only (the site is French/English).
**Source:** the Google Fonts `css2` API, which serves the upstream releases of
[Space Grotesk](https://github.com/floriankarsten/space-grotesk) and
[IBM Plex Mono](https://github.com/IBM/plex).
**License:** both families are under the [SIL Open Font License 1.1](https://openfontlicense.org/),
which explicitly permits redistribution — bundling them here is allowed.

They are declared with `@font-face` at the top of `site/styles.css`. To add a
weight, add the file here and a matching `@font-face` block there. If a file is
ever missing, the page falls back to the system fonts listed in `--sans` /
`--mono`, so nothing breaks — it just looks like the OS default.
