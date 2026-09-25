import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import test from "node:test";
import { analyses, cli, userGuide } from "../scripts/site/catalog.mjs";
import { guidePage } from "../scripts/site/template.mjs";

const root = resolve(import.meta.dirname, "..");

test("homepage metadata matches its French content and social preview", async () => {
  const home = await readFile(resolve(root, "site/index.html"), "utf8");

  assert.match(home, /<html lang="fr">/);
  assert.match(home, /<title>FlacCompagnon — Vérifiez vos fichiers FLAC/);
  assert.match(home, /<meta property="og:image:alt"/);
  assert.match(home, /<meta name="twitter:image:alt"/);
  assert.match(
    home,
    /hreflang="en" href="https:\/\/craft-and-code\.github\.io\/FlacCompagnon\/en\//,
  );
  assert.match(home, /"@type": "SoftwareApplication"/);
  assert.doesNotMatch(home, /name="keywords"/);
});

test("documentation pages expose localized sharing metadata and breadcrumbs", () => {
  const page = {
    slug: "integrated-loudness",
    title: { fr: "LUFS intégré", en: "Integrated LUFS" },
    description: { fr: "Loudness du programme.", en: "Programme loudness." },
    group: "loudness",
  };
  const html = guidePage(page, "fr", { html: "<p>Contenu.</p>", headings: [] });

  assert.match(html, /<html lang="fr">/);
  assert.match(html, /<meta name="robots" content="index,follow"\/>/);
  assert.match(
    html,
    /<link rel="alternate" hreflang="en" href="https:\/\/craft-and-code\.github\.io\/FlacCompagnon\/docs\/en\/integrated-loudness\.html"\/>/,
  );
  assert.match(
    html,
    /<meta property="og:url" content="https:\/\/craft-and-code\.github\.io\/FlacCompagnon\/docs\/fr\/integrated-loudness\.html"\/>/,
  );
  assert.match(html, /<meta name="twitter:card" content="summary_large_image"\/>/);

  const [schema] = [...html.matchAll(/<script type="application\/ld\+json">([^<]+)<\/script>/g)];
  assert.ok(schema, "Expected a JSON-LD block");
  const breadcrumb = JSON.parse(schema[1]);
  assert.equal(breadcrumb["@type"], "BreadcrumbList");
  assert.equal(breadcrumb.itemListElement.at(-1).name, "LUFS intégré");
});

test("the user guide has its own localized navigation section", () => {
  const html = guidePage(userGuide, "fr", { html: "<p>Contenu.</p>", headings: [] });

  assert.match(html, /data-guide-nav-section="analyses"/);
  assert.match(html, /data-guide-nav-section="cli"/);
  assert.match(html, /data-guide-nav-section="user-guide" open/);
  assert.match(html, /Prise en main/);
  assert.ok(
    html.indexOf('data-guide-nav-section="user-guide"') <
      html.indexOf('data-guide-nav-section="analyses"'),
  );
  assert.match(html, /<div class="guide-api-block"><a class="guide-api"/);
});

test("the CLI and user guide are outside the analysis sequence", () => {
  for (const page of [cli, userGuide]) {
    const html = guidePage(page, "fr", { html: "<p>Contenu.</p>", headings: [] });

    assert.doesNotMatch(html, /guide-neighbors/);
    assert.doesNotMatch(html, /guide-api-note/);
  }
});

test("analysis neighbours never lead into the guide or command line", () => {
  const html = guidePage(analyses[0], "fr", { html: "<p>Contenu.</p>", headings: [] });

  assert.match(html, /guide-neighbors/);
  assert.doesNotMatch(html, /guide-neighbors[\s\S]*?cli\.html/);
  assert.doesNotMatch(html, /guide-neighbors[\s\S]*?user-guide\.html/);
});

test("the homepage CLI example is one terminal surface", async () => {
  const home = await readFile(resolve(root, "site/index.html"), "utf8");

  assert.match(home, /<pre>\s*flaccompagnon "Music\/Album"/);
  assert.doesNotMatch(home, /<pre>\s*<code>\s*flaccompagnon/);
});
