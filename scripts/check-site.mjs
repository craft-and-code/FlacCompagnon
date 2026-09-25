// Validate the complete generated site, including GitHub Pages subpath links.
import assert from "node:assert/strict";
import { readFile, readdir, stat } from "node:fs/promises";
import { resolve, relative, dirname } from "node:path";
import { externalDocumentationPrefixes, pages } from "./site/catalog.mjs";

const output = resolve(import.meta.dirname, "../dist-site");
async function walk(dir) {
  const entries = await readdir(dir, { withFileTypes: true });
  return (
    await Promise.all(
      entries.map((e) => (e.isDirectory() ? walk(resolve(dir, e.name)) : resolve(dir, e.name))),
    )
  ).flat();
}
const files = await walk(output);
const html = new Map(
  await Promise.all(
    files.filter((f) => f.endsWith(".html")).map(async (f) => [f, await readFile(f, "utf8")]),
  ),
);
const errors = [];
let linkCount = 0;
const homepage = html.get(resolve(output, "index.html"));
assert.ok(homepage, "Missing homepage");
for (const markup of [
  '<meta name="robots" content="index, follow"',
  '<meta property="og:image:alt"',
  '<meta name="twitter:image:alt"',
  '"@type": "SoftwareApplication"',
]) {
  assert.ok(homepage.includes(markup), `Homepage misses ${markup}`);
}
const englishHomepage = html.get(resolve(output, "en/index.html"));
assert.ok(englishHomepage, "Missing English homepage");
for (const markup of [
  '<html lang="en" data-docs-root="../docs" data-default-language="en">',
  '<link rel="canonical" href="https://craft-and-code.github.io/FlacCompagnon/en/"',
  'href="../docs/en/index.html"',
  'src="../app.js"',
]) {
  assert.ok(englishHomepage.includes(markup), `English homepage misses ${markup}`);
}
for (const [file, source] of html) {
  const ids = [...source.matchAll(/\bid="([^"]+)"/g)].map((m) => m[1]);
  if (new Set(ids).size !== ids.length) errors.push(`${relative(output, file)}: duplicate IDs`);
  if ([...source.matchAll(/<h1(?:\s|>)/g)].length !== 1)
    errors.push(`${relative(output, file)}: expected one h1`);
  for (const href of externalDocumentationPrefixes) {
    for (const anchor of source.matchAll(/<a\b[^>]*>/g)) {
      if (
        anchor[0].includes(`href="${href}"`) &&
        !/target="_blank"\s+rel="noopener"/.test(anchor[0])
      ) {
        errors.push(`${relative(output, file)}: ${href} must open in a new tab`);
      }
    }
  }
  for (const match of source.matchAll(/\b(?:href|src)="([^"]+)"/g)) {
    const url = match[1];
    if (/^(?:[a-z]+:|\/\/)/i.test(url)) continue;
    linkCount++;
    const [path, fragment] = url.split("#");
    let target = path ? resolve(dirname(file), decodeURIComponent(path)) : file;
    if (relative(output, target).startsWith("..")) {
      errors.push(`${file}: link escapes site: ${url}`);
      continue;
    }
    try {
      if ((await stat(target)).isDirectory()) target = resolve(target, "index.html");
      await stat(target);
      if (
        fragment &&
        html.has(target) &&
        !html.get(target).includes(`id="${decodeURIComponent(fragment)}"`)
      ) {
        errors.push(`${relative(output, file)}: missing anchor ${url}`);
      }
    } catch {
      errors.push(`${relative(output, file)}: missing file ${url}`);
    }
  }
}
for (const lang of ["fr", "en"]) {
  for (const page of pages) {
    const source = html.get(resolve(output, `docs/${lang}/${page.slug}.html`));
    assert.ok(source, `Missing ${lang} page: ${page.slug}`);
    assert.ok(source.includes(`<html lang="${lang}">`));
    assert.ok(
      source.includes(`../${lang === "fr" ? "en" : "fr"}/${page.slug}.html`),
      "Missing language counterpart",
    );
    assert.ok(source.includes("Rustdoc"), "Missing API link");
    for (const markup of [
      '<meta name="robots" content="index,follow"',
      '<meta property="og:url"',
      '<meta name="twitter:card" content="summary_large_image"',
      '"@type":"BreadcrumbList"',
    ]) {
      assert.ok(source.includes(markup), `${lang}/${page.slug} misses ${markup}`);
    }
    const [schema] = [
      ...source.matchAll(/<script type="application\/ld\+json">([^<]+)<\/script>/g),
    ];
    assert.ok(schema, `${lang}/${page.slug} misses structured data`);
    const breadcrumb = JSON.parse(schema[1]);
    assert.equal(breadcrumb["@type"], "BreadcrumbList");
    assert.equal(
      breadcrumb.itemListElement.at(-1).name,
      page.slug === "index" ? "Documentation" : page.title[lang],
    );
  }
}
// Keep the published option table in sync with the actual command parser.
const cliSource = await readFile(resolve(import.meta.dirname, "../cli/src/main.rs"), "utf8");
const analysisNames = [
  ...cliSource.match(/const ANALYSES:.*?= &\[([\s\S]*?)\];/)[1].matchAll(/"([^"]+)"/g),
].map((m) => m[1]);
for (const lang of ["fr", "en"]) {
  const guide = html.get(resolve(output, `docs/${lang}/cli.html`));
  for (const name of analysisNames)
    assert.ok(guide.includes(`<code>${name}</code>`), `CLI guide misses ${name}`);
}
assert.equal(errors.length, 0, errors.join("\n"));
console.log(
  `Checked ${html.size} HTML pages and ${linkCount} local links; both CLI option lists match the parser.`,
);
