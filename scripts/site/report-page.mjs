// The reader is isolated from the landing-page animations and cannot call an API or upload a report.
export function reportPage(lang) {
  const fr = lang === "fr";
  const root = fr ? "" : "../";
  const title = fr ? "Consulter une analyse" : "Read an analysis";
  const description = fr
    ? "Déposez un rapport JSON FlacCompagnon pour consulter ses pistes et ses mesures. Lecture locale, sans envoi de données à un serveur."
    : "Drop a FlacCompagnon JSON report to explore its tracks and measurements. Local viewing, with no data sent to a server.";
  return `<!doctype html>
<html lang="${lang}"><head>
<meta charset="utf-8"/><meta name="viewport" content="width=device-width,initial-scale=1"/>
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self'; img-src 'self'; connect-src 'none'; form-action 'none'; base-uri 'none'; object-src 'none'"/>
<title>${title} — FlacCompagnon</title><meta name="description" content="${description}"/>
<meta name="robots" content="index,follow"/><meta name="theme-color" content="#0d0e13"/>
<link rel="canonical" href="https://craft-and-code.github.io/FlacCompagnon/${fr ? "" : "en/"}report.html"/>
<link rel="alternate" hreflang="fr" href="${root}report.html"/>
<link rel="alternate" hreflang="en" href="${root}en/report.html"/>
<link rel="icon" href="${root}assets/favicon.ico" sizes="any"/>
<link rel="stylesheet" href="${root}styles.css"/>
<link rel="stylesheet" href="${root}vendor/phemios-analysis/flac-analysis-report.css"/>
<link rel="stylesheet" href="${root}report-viewer.css"/>
</head><body>
<a class="skip-link" href="#content">${fr ? "Aller au contenu" : "Skip to content"}</a>
<header class="nav"><a class="brand" href="./"><img src="${root}assets/logo.svg" width="28" height="28" alt="FlacCompagnon"/><span>FlacCompagnon</span></a>
<div class="nav-right"><a class="nav-docs" href="${root}docs/${lang}/user-guide.html">Docs ↗</a><nav class="lang" aria-label="${fr ? "Langue" : "Language"}"><a href="${root}report.html" lang="fr"${fr ? ' aria-current="page"' : ""}>FR</a><a href="${root}en/report.html" lang="en"${fr ? "" : ' aria-current="page"'}>EN</a></nav></div></header>
<main id="content" class="report-page"><p class="kicker">JSON · ${fr ? "Lecture locale" : "Local viewing"}</p>
<h1>${title}</h1><p class="lead">${fr ? "Retrouvez les résultats de FlacCompagnon, piste par piste. Cette page affiche le rapport existant ; elle ne lit pas vos fichiers audio et ne relance aucune analyse." : "Explore FlacCompagnon results, track by track. This page displays the existing report; it does not read your audio files or run a new analysis."}</p>
<flac-report-viewer></flac-report-viewer>
<noscript><p>${fr ? "Activez JavaScript pour lire votre rapport localement dans ce navigateur." : "Enable JavaScript to read your report locally in this browser."}</p></noscript>
</main><footer class="footer"><a href="./">← ${fr ? "Retour au site" : "Back to the website"}</a><a href="${root}vendor/phemios-analysis/README.md">${fr ? "Sources et licences" : "Source and licenses"}</a></footer>
<script type="module" src="${root}report-viewer.js"></script></body></html>`;
}
