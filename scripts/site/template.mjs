import {
  aede,
  analyses,
  baseUrl,
  cli,
  escapeHtml as esc,
  groups,
  pages,
  repository,
  rustdoc,
  userGuide,
} from "./catalog.mjs";
import { diagram } from "./diagrams.mjs";
import { explainers } from "./explainers.mjs";

export function articleList(lang, homepage = false) {
  const heading = homepage ? "h3" : "h2";
  return groups
    .map(
      (group) =>
        `<section class="analysis-group" id="${homepage ? "analysis-" : ""}${group.id}"><${heading} ${homepage ? `data-fr="${esc(group.fr)}" data-en="${esc(group.en)}"` : ""}>${esc(group[lang])}</${heading}><ul>${analyses
          .filter((page) => page.group === group.id)
          .map(
            (page) =>
              `<li><a ${homepage ? `data-doc="${page.slug}"` : ""} href="${homepage ? "docs/fr/" : ""}${page.slug}.html"><span ${homepage ? `data-fr="${esc(page.title.fr)}" data-en="${esc(page.title.en)}"` : ""}>${esc(page.title[lang])}</span><span aria-hidden="true">↗</span></a>${homepage ? "" : `<p>${esc(page.description[lang])}</p>`}</li>`,
          )
          .join("")}</ul></section>`,
    )
    .join("");
}

function languageLinks(page) {
  return ["fr", "en"]
    .map(
      (language) =>
        `<link rel="alternate" hreflang="${language}" href="${baseUrl}docs/${language}/${page.slug}.html"/>`,
    )
    .join("");
}

function breadcrumbSchema(page, lang, title) {
  const documentationUrl = `${baseUrl}docs/${lang}/index.html`;
  const items = [
    { "@type": "ListItem", position: 1, name: "FlacCompagnon", item: baseUrl },
    {
      "@type": "ListItem",
      position: 2,
      name: "Documentation",
      ...(page.slug === "index" ? {} : { item: documentationUrl }),
    },
  ];
  if (page.slug !== "index") items.push({ "@type": "ListItem", position: 3, name: title });
  return JSON.stringify({
    "@context": "https://schema.org",
    "@type": "BreadcrumbList",
    itemListElement: items,
  }).replace(/</g, "\\u003c");
}

function pageHead(page, lang, title, description) {
  const isFrench = lang === "fr";
  const pageUrl = `${baseUrl}docs/${lang}/${page.slug}.html`;
  const documentTitle =
    page.slug === "index" ? (isFrench ? "Guide d’analyse audio" : "Audio analysis guide") : title;
  const locale = isFrench ? "fr_FR" : "en_US";
  const alternateLocale = isFrench ? "en_US" : "fr_FR";
  const imageAlt = isFrench
    ? "FlacCompagnon, outil d’analyse audio"
    : "FlacCompagnon audio analysis tool";
  return `<head>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width,initial-scale=1"/>
    <title>${esc(documentTitle)} — FlacCompagnon Docs</title>
    <meta name="description" content="${esc(description)}"/>
    <meta name="robots" content="index,follow"/>
    <meta name="theme-color" content="#0d0e13"/>
    <link rel="canonical" href="${pageUrl}"/>
    ${languageLinks(page)}
    <link rel="alternate" hreflang="x-default" href="${baseUrl}docs/en/${page.slug}.html"/>
    <meta property="og:type" content="article"/>
    <meta property="og:url" content="${pageUrl}"/>
    <meta property="og:site_name" content="FlacCompagnon"/>
    <meta property="og:title" content="${esc(documentTitle)} — FlacCompagnon"/>
    <meta property="og:description" content="${esc(description)}"/>
    <meta property="og:image" content="${baseUrl}assets/og-image.png"/>
    <meta property="og:image:alt" content="${imageAlt}"/>
    <meta property="og:locale" content="${locale}"/>
    <meta property="og:locale:alternate" content="${alternateLocale}"/>
    <meta name="twitter:card" content="summary_large_image"/>
    <meta name="twitter:title" content="${esc(documentTitle)} — FlacCompagnon"/>
    <meta name="twitter:description" content="${esc(description)}"/>
    <meta name="twitter:image" content="${baseUrl}assets/og-image.png"/>
    <meta name="twitter:image:alt" content="${imageAlt}"/>
    <script type="application/ld+json">${breadcrumbSchema(page, lang, title)}</script>
    <link rel="icon" href="../../assets/favicon.ico"/>
    <link rel="stylesheet" href="../../styles.css"/>
    <link rel="stylesheet" href="../../directory.css"/>
    <link rel="stylesheet" href="../../guide.css"/>
    <script src="../../guide.js" type="module"></script>
  </head>`;
}

function navigationSection({ id, title, content, current }) {
  return `<details class="guide-nav-section" data-guide-nav-section="${id}"${current ? " open" : ""}><summary class="guide-nav-section-title">${title}<span aria-hidden="true"></span></summary>${content}</details>`;
}

function sidebar(lang, slug) {
  const isFrench = lang === "fr";
  const link = (page) =>
    `<li><a href="${page.slug}.html" ${page.slug === slug ? 'aria-current="page"' : ""}>${esc(page.title[lang])}</a></li>`;
  const analysisGroups = groups
    .map(
      (group) =>
        `<div class="guide-nav-group"><h2>${esc(group[lang])}</h2><ul>${analyses
          .filter((page) => page.group === group.id)
          .map(link)
          .join("")}</ul></div>`,
    )
    .join("");
  const sections = [
    navigationSection({
      id: "user-guide",
      title: isFrench ? "Guide de l’utilisateur" : "User guide",
      content: `<ul>${link(userGuide)}</ul>`,
      current: slug === userGuide.slug,
    }),
    navigationSection({
      id: "analyses",
      title: isFrench ? "Analyses" : "Analyses",
      content: `<a class="guide-overview" href="index.html" ${slug === "index" ? 'aria-current="page"' : ""}>${isFrench ? "Vue d’ensemble" : "Overview"}</a>${analysisGroups}`,
      current: slug === "index" || analyses.some((page) => page.slug === slug),
    }),
    navigationSection({
      id: "cli",
      title: isFrench ? "Ligne de commande" : "Command line",
      content: `<ul>${link(cli)}</ul>`,
      current: slug === cli.slug,
    }),
  ].join("");
  return `<aside class="guide-sidebar"><nav class="guide-nav" aria-label="Documentation">${sections}<div class="guide-api-block"><a class="guide-api" href="${rustdoc}" target="_blank" rel="noopener">Rustdoc · API Rust ↗</a></div></nav></aside>`;
}

export function guidePage(page, lang, rendered = {}) {
  const isFrench = lang === "fr";
  const index = page.slug === "index";
  const title = index
    ? isFrench
      ? "Comprendre votre audio."
      : "Understand your audio."
    : page.title[lang];
  const description = index
    ? isFrench
      ? "Des repères simples aux méthodes détaillées : découvrez ce que chaque analyse observe, comment lire ses résultats et où se situent ses limites."
      : "From simple explanations to detailed methods: discover what each analysis observes, how to read its results and where its limits lie."
    : page.description[lang];
  const beginner = explainers[page.slug];
  const body = index
    ? `<p class="guide-intro">${description}</p><div class="guide-callout"><strong>${isFrench ? "Un résultat est un repère, pas une note de qualité." : "A result is a clue, not a quality grade."}</strong><p>${isFrench ? "Clean signifie qu’aucun détecteur n’a levé d’alerte. Un tiret indique une mesure indisponible. Écoutez les passages signalés et lisez les limites avant de conclure." : "Clean means no detector raised a finding. A dash means a measurement is unavailable. Audition flagged passages and read the limits before drawing a conclusion."}</p></div><a class="guide-cli-link" href="cli.html"><span><strong>${isFrench ? "Commencer avec la ligne de commande" : "Get started with the command line"}</strong><small>${isFrench ? "Installation · exemples · JSON · Aède" : "Installation · examples · JSON · Aède"}</small></span><span aria-hidden="true">→</span></a><label class="guide-search" hidden>${isFrench ? "Trouver une analyse" : "Find an analysis"}<input type="search" placeholder="${isFrench ? "Phase, LUFS, clipping…" : "Phase, LUFS, clipping…"}" autocomplete="off" aria-controls="analysis-directory"/></label><p class="guide-search-status" role="status" aria-live="polite"></p><div id="analysis-directory" class="analysis-directory">${articleList(lang)}</div>`
    : beginner
      ? `<div class="guide-intro">${esc(beginner[0][isFrench ? 0 : 1])}</div><section class="guide-reading"><h2 id="reading">${isFrench ? "Comment lire le résultat" : "Reading the result"}</h2><p>${esc(beginner[1][isFrench ? 0 : 1])}</p></section>${diagram(page.slug, lang)}<details class="technical-details" id="technical"><summary><span>${isFrench ? "Pour aller plus loin" : "Go deeper"}</span><small>${isFrench ? "Méthode, limites, vérifications et sources" : "Method, limits, verification and sources"}</small></summary><div class="guide-prose">${rendered.html}</div></details>`
      : `<p class="guide-intro">${esc(description)}</p><div class="guide-prose">${rendered.html}</div>`;
  const currentAnalysis = analyses.findIndex((item) => item.slug === page.slug);
  const isAnalysis = currentAnalysis !== -1;
  const neighbors = !isAnalysis
    ? ""
    : `<nav class="guide-neighbors" aria-label="${isFrench ? "Pages voisines" : "Adjacent pages"}">${[
        analyses[currentAnalysis - 1],
        analyses[currentAnalysis + 1],
      ]
        .map((item, position) =>
          item
            ? `<a href="${item.slug}.html"><small>${position ? (isFrench ? "Suivant →" : "Next →") : isFrench ? "← Précédent" : "← Previous"}</small>${esc(item.title[lang])}</a>`
            : "<span></span>",
        )
        .join("")}</nav>`;
  const apiNote = !isAnalysis
    ? ""
    : `<div class="guide-api-note"><p>${isFrench ? "Vous intégrez le moteur dans un logiciel ?" : "Integrating the engine in your software?"}</p><a class="btn btn-outline" href="${rustdoc}" target="_blank" rel="noopener">${isFrench ? "Consulter l’API Rust dans Rustdoc" : "Browse the Rust API in Rustdoc"} ↗</a></div>`;
  const toc = (rendered.headings || [])
    .map((heading) => `<a href="#${heading.id}">${esc(heading.title)}</a>`)
    .join("");
  const languageSelector = ["en", "fr"]
    .map(
      (language) =>
        `<a href="../${language}/${page.slug}.html" lang="${language}" hreflang="${language}" data-language="${language}" ${lang === language ? 'aria-current="page"' : ""}>${language.toUpperCase()}</a>`,
    )
    .join("");
  const kicker = index
    ? "Documentation"
    : page.slug === cli.slug
      ? isFrench
        ? "En pratique"
        : "Hands-on"
      : page.slug === userGuide.slug
        ? isFrench
          ? "Prise en main"
          : "Getting started"
        : esc(groups.find((group) => group.id === page.group)[lang]);
  return `<!doctype html><html lang="${lang}">${pageHead(page, lang, title, description)}<body class="guide-body"><a class="skip-link" href="#content">${isFrench ? "Aller au contenu" : "Skip to content"}</a><header class="nav guide-header"><a class="brand" href="../../"><img src="../../assets/logo.png" alt="" width="28" height="28"/><span>FlacCompagnon</span></a><a class="guide-header-label" href="index.html">Docs</a><div class="nav-right"><a class="guide-rustdoc" href="${rustdoc}" target="_blank" rel="noopener">Rustdoc ↗</a><nav class="lang" aria-label="${isFrench ? "Langue" : "Language"}">${languageSelector}</nav></div></header><div class="guide-layout">${sidebar(lang, page.slug)}<main id="content" class="guide-main"><p class="kicker">${kicker}</p><h1>${esc(title)}</h1>${body}${neighbors}${apiNote}</main><aside class="guide-toc"><p>${isFrench ? "Sur cette page" : "On this page"}</p>${beginner ? `<a href="#reading">${isFrench ? "Lire le résultat" : "Reading the result"}</a><a href="#technical">${isFrench ? "Aller plus loin" : "Go deeper"}</a>` : ""}${toc || (index ? groups.map((group) => `<a href="#${group.id}">${esc(group[lang])}</a>`).join("") : "")}</aside></div><footer class="footer"><span>${isFrench ? "FlacCompagnon · Application MIT · Core MPL-2.0 · Rust + Tauri" : "FlacCompagnon · App MIT · Core MPL-2.0 · Rust + Tauri"}</span><span class="ft-links"><a href="../../">${isFrench ? "Accueil" : "Home"}</a><a href="${repository}">GitHub</a><a href="${aede}" target="_blank" rel="noopener">Aède</a></span></footer></body></html>`;
}
