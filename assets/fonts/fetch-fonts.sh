#!/usr/bin/env bash
#
# Download the self-hosted web fonts (Space Grotesk + IBM Plex Mono, latin
# subset, one .woff2 per weight) into this folder. Run once — the resulting
# .woff2 files are what styles.css references via @font-face.
#
#   cd site/assets/fonts && ./fetch-fonts.sh
#
# Both families are licensed under the SIL Open Font License 1.1, which permits
# redistribution — so committing the .woff2 files to the repo is fine.
#
# Why a script instead of committing the files directly: it documents exactly
# which weights/subsets are used and lets you re-fetch or add a weight later.

set -euo pipefail
cd "$(dirname "$0")"

UA="Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36"

# Pull the single "latin" .woff2 URL out of a Google Fonts css2 response.
latin_url() {
  # The css2 API returns one @font-face block per unicode-range subset, each
  # preceded by a "/* latin */"-style comment. Grab the first woff2 URL that
  # appears after the "/* latin */" marker.
  awk '/\/\* latin \*\//{f=1} f && match($0, /https:[^)]+\.woff2/){print substr($0, RSTART, RLENGTH); exit}'
}

fetch() {
  local family="$1" weight="$2" out="$3"
  local css url
  css=$(curl -fsSL -A "$UA" \
    "https://fonts.googleapis.com/css2?family=${family}:wght@${weight}&display=swap")
  url=$(printf '%s\n' "$css" | latin_url)
  if [ -z "${url:-}" ]; then
    echo "  ! could not find a latin woff2 for ${family} ${weight}" >&2
    return 1
  fi
  curl -fsSL -A "$UA" "$url" -o "$out"
  printf '  %s  (%s bytes)\n' "$out" "$(wc -c < "$out")"
}

echo "Space Grotesk:"
for w in 400 500 600 700; do fetch "Space+Grotesk" "$w" "space-grotesk-$w.woff2"; done
echo "IBM Plex Mono:"
for w in 400 500; do fetch "IBM+Plex+Mono" "$w" "ibm-plex-mono-$w.woff2"; done
echo "Done. The site now uses local fonts — no external requests."
