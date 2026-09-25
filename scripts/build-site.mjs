import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pages, baseUrl } from "./site/catalog.mjs";
import { renderMarkdown } from "./site/markdown.mjs";
import { guidePage, articleList } from "./site/template.mjs";
import { DICT } from "../site/copy.js";

const root = resolve(import.meta.dirname, "..");
const output = resolve(root, "dist-site");
await rm(output, { recursive: true, force: true });
await cp(resolve(root, "site"), output, { recursive: true });
const pkg = JSON.parse(await readFile(resolve(root, "package.json"), "utf8"));
const escapeText = (value) =>
  value.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

function localizeHome(source, language) {
  let home = source.replace("<!-- ANALYSIS_DIRECTORY -->", articleList(language, true));
  home = home.replace(
    /<([a-z][a-z0-9]*)\b([^>]*\bdata-i18n(-html)?="([^"]+)"[^>]*)>[\s\S]*?<\/\1\s*>/g,
    (match, tag, attrs, markup, key) => {
      const value = DICT[language][key];
      if (value == null) return match;
      const content = markup ? value : escapeText(value);
      return `<${tag}${attrs}>${content}</${tag}>`;
    },
  );
  return home.replace(
    /<([a-z][a-z0-9]*)\b([^>]*\bdata-fr="([^"]*)"[^>]*\bdata-en="([^"]*)"[^>]*)>[\s\S]*?<\/\1\s*>/g,
    (match, tag, attrs, french, english) =>
      `<${tag}${attrs}>${language === "fr" ? french : english}</${tag}>`,
  );
}

function englishHome(source) {
  return source
    .replace(
      '<html lang="fr">',
      '<html lang="en" data-docs-root="../docs" data-default-language="en">',
    )
    .replace(
      "FlacCompagnon — Vérifiez vos fichiers FLAC et détectez les faux lossless",
      "FlacCompagnon — Verify FLAC files and detect fake lossless audio",
    )
    .replace(
      "FlacCompagnon vérifie l’authenticité de vos fichiers FLAC : faux lossless, upsampling, transcodage, loudness, phase stéréo, dynamique et intégrité. Application gratuite, CLI et moteur Rust.",
      "FlacCompagnon checks FLAC authenticity: fake lossless, upsampling, transcoding, loudness, stereo phase, dynamics and integrity. A free native app, CLI and Rust engine.",
    )
    .replaceAll(
      "FlacCompagnon — Vérifiez vos fichiers FLAC",
      "FlacCompagnon — Verify your FLAC files",
    )
    .replace(
      "Détectez les faux lossless, analysez le loudness, la phase stéréo, la dynamique et l’intégrité de votre bibliothèque musicale.",
      "Detect fake lossless files and analyze the loudness, stereo phase, dynamics and integrity of your music library.",
    )
    .replace(
      "Détectez les faux lossless et analysez l’authenticité de votre bibliothèque musicale.",
      "Detect fake lossless files and analyze the authenticity of your music library.",
    )
    .replaceAll(
      "FlacCompagnon, outil d’analyse de fichiers audio",
      "FlacCompagnon audio analysis tool",
    )
    .replace(
      '<link rel="canonical" href="https://craft-and-code.github.io/FlacCompagnon/" />',
      '<link rel="canonical" href="https://craft-and-code.github.io/FlacCompagnon/en/" />',
    )
    .replace(
      '<meta property="og:url" content="https://craft-and-code.github.io/FlacCompagnon/" />',
      '<meta property="og:url" content="https://craft-and-code.github.io/FlacCompagnon/en/" />',
    )
    .replace(
      '"url": "https://craft-and-code.github.io/FlacCompagnon/"',
      '"url": "https://craft-and-code.github.io/FlacCompagnon/en/"',
    )
    .replace(
      '<meta property="og:locale" content="fr_FR" />',
      '<meta property="og:locale" content="en_US" />',
    )
    .replace(
      '<meta property="og:locale:alternate" content="en_US" />',
      '<meta property="og:locale:alternate" content="fr_FR" />',
    )
    .replace(
      "Détection de faux lossless, upscaling, upsampling et transcodage ; loudness LUFS ; phase et balance stéréo ; dynamique, clipping et true peak ; vérification MD5 FLAC ; CLI ; export JSON.",
      "Fake-lossless, upscaling, upsampling and transcoding detection; LUFS loudness; stereo phase and balance; dynamics, clipping and true peak; FLAC MD5 verification; CLI; JSON export.",
    )
    .replace(/((?:href|src)=")(assets\/|styles\.css|directory\.css|home\.css|app\.js)/g, "$1../$2")
    .replaceAll('href="docs/fr/', 'href="../docs/en/');
}

const homeSource = await readFile(resolve(output, "index.html"), "utf8");
const frenchHome = localizeHome(homeSource, "fr").replace(
  /"softwareVersion": "[^"]+"/,
  `"softwareVersion": "${pkg.version}"`,
);
const englishLocalizedHome = localizeHome(homeSource, "en").replace(
  /"softwareVersion": "[^"]+"/,
  `"softwareVersion": "${pkg.version}"`,
);
await writeFile(resolve(output, "index.html"), frenchHome);
await mkdir(resolve(output, "en"), { recursive: true });
await writeFile(resolve(output, "en", "index.html"), englishHome(englishLocalizedHome));
const urls = [baseUrl, `${baseUrl}en/`];
for (const lang of ["fr", "en"]) {
  const dir = resolve(output, "docs", lang);
  await mkdir(dir, { recursive: true });
  await writeFile(resolve(dir, "index.html"), guidePage({ slug: "index" }, lang));
  urls.push(`${baseUrl}docs/${lang}/index.html`);
  for (const page of pages) {
    const source = await readFile(
      resolve(root, "docs", ...(lang === "fr" ? ["fr"] : []), `${page.slug}.md`),
      "utf8",
    );
    await writeFile(
      resolve(dir, `${page.slug}.html`),
      guidePage(page, lang, renderMarkdown(source, lang)),
    );
    urls.push(`${baseUrl}docs/${lang}/${page.slug}.html`);
  }
}
await writeFile(
  resolve(output, "docs", "index.html"),
  `<!doctype html><html lang="en"><head><meta charset="utf-8"/><meta name="viewport" content="width=device-width,initial-scale=1"/><title>Documentation language selection — FlacCompagnon</title><meta name="robots" content="noindex,follow"/><link rel="stylesheet" href="../styles.css"/></head><body><main class="section"><h1>FlacCompagnon · Documentation</h1><p class="lead"><a class="btn btn-outline" href="fr/index.html" lang="fr">Français →</a> <a class="btn btn-outline" href="en/index.html" lang="en">English →</a></p></main></body></html>`,
);
await writeFile(
  resolve(output, "sitemap.xml"),
  `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">${urls.map((url) => `<url><loc>${url}</loc></url>`).join("")}</urlset>\n`,
);
console.log(
  `Built French and English landing pages and ${pages.length + 1} documentation pages in each language → dist-site/`,
);
