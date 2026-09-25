import { escapeHtml as esc } from "./catalog.mjs";

const line = (x1, y1, x2, y2, cls = "diagram-axis") =>
  `<line class="${cls}" x1="${x1}" y1="${y1}" x2="${x2}" y2="${y2}"/>`;
const label = (x, y, value, cls = "") =>
  `<text x="${x}" y="${y}" class="${cls}">${esc(value)}</text>`;
const path = (fn, start = 50, end = 610, step = 2, cls = "diagram-primary") =>
  `<path class="${cls}" d="${Array.from(
    { length: Math.floor((end - start) / step) + 1 },
    (_, i) => {
      const x = start + i * step;
      return `${i ? "L" : "M"}${x},${fn(x).toFixed(2)}`;
    },
  ).join(" ")}"/>`;

export function diagram(slug, lang) {
  const fr = lang === "fr";
  // Generic checksum flows do not add useful context to either hash page.
  if (["flac-md5", "fingerprints"].includes(slug)) return "";
  let drawing = "",
    caption = "";
  if (
    ["integrated-loudness", "momentary-loudness", "short-term-loudness", "loudness-range"].includes(
      slug,
    )
  ) {
    drawing =
      label(
        50,
        30,
        fr
          ? "Un même passage, plusieurs durées d’observation"
          : "One passage, different observation windows",
      ) +
      line(50, 160, 610, 160) +
      path((x) => 157 - (x > 310 && x < 390 ? 90 : 30) * (0.8 + 0.2 * Math.sin(x * 0.14))) +
      `<rect class="diagram-window" x="310" y="52" width="37" height="112"/>` +
      line(200, 193, 480, 193, "diagram-secondary") +
      label(310, 46, "M · 400 ms") +
      label(220, 217, "S · 3 s") +
      label(
        50,
        245,
        fr
          ? "Intégré : ensemble du programme · LRA : variation des fenêtres S"
          : "Integrated: whole programme · LRA: variation of S windows",
      );
    caption = fr
      ? "Schéma de principe : la fenêtre M réagit à un accent bref ; S l’étale sur trois secondes. L’intégré et la LRA appliquent leurs propres portes de mesure."
      : "Conceptual diagram: M captures a brief accent; S averages it over three seconds. Integrated loudness and LRA apply their own gates.";
  } else if (["stereo-polarity", "local-phase", "fake-stereo"].includes(slug)) {
    const opposed = slug !== "fake-stereo";
    drawing =
      label(50, 30, opposed ? (fr ? "Deux canaux opposés" : "Two opposed channels") : "Dual mono") +
      label(22, 89, "L") +
      label(22, 169, "R") +
      line(50, 85, 610, 85) +
      line(50, 165, 610, 165) +
      path((x) => 85 - 28 * Math.sin(((x - 50) * Math.PI) / 70)) +
      path(
        (x) => 165 - (opposed ? -28 : 28) * Math.sin(((x - 50) * Math.PI) / 70),
        50,
        610,
        2,
        "diagram-secondary",
      ) +
      label(
        50,
        225,
        opposed
          ? fr
            ? "Somme mono à gains égaux : annulation"
            : "Equal-gain mono sum: cancellation"
          : fr
            ? "Même signal dans les deux canaux"
            : "Same signal in both channels",
      );
    caption = fr
      ? "Sinusoïdes illustratives à gains égaux. La corrélation va de −1 (opposition) à +1 (même polarité). Un mélange réel peut avoir des relations différentes selon le moment ou la fréquence."
      : "Illustrative equal-gain sine waves. Correlation runs from −1 (opposition) to +1 (matching polarity). Real mixes can differ by time or frequency.";
  } else if (slug === "high-frequency-stereo" || slug === "stereo-balance") {
    const hf = slug === "high-frequency-stereo";
    drawing =
      label(
        50,
        32,
        hf
          ? fr
            ? "Énergie Side par rapport à Mid"
            : "Side energy relative to Mid"
          : fr
            ? "Niveau RMS par canal"
            : "RMS level per channel",
      ) +
      label(50, 90, hf ? "1.5–5 kHz" : "L") +
      label(50, 170, hf ? "6–20 kHz" : "R") +
      `<rect class="diagram-bar" x="180" y="66" width="400" height="32"/><rect class="diagram-bar secondary" x="180" y="146" width="${hf ? 100 : 200}" height="32"/>` +
      label(180, 125, hf ? "0 dB" : "0 dB") +
      label(180, 205, hf ? "−30 dB" : "−6 dB");
    caption = hf
      ? fr
        ? "Exemple schématique : des aigus beaucoup plus centrés que la bande de référence. Les barres illustrent les niveaux indiqués, sans reproduire une mesure de musique."
        : "Schematic example: treble much more centred than the reference band. Bars illustrate the labelled levels, not a measurement of music."
      : fr
        ? "Exemple : une amplitude RMS divisée par deux à droite donne environ 6 dB d’écart. L’échelle des barres est linéaire en amplitude."
        : "Example: halving right-channel RMS amplitude gives a difference of about 6 dB. Bars use a linear amplitude scale.";
  } else if (
    ["clipping", "true-peak", "dc-offset", "impulses", "dropouts", "dynamic-range"].includes(slug)
  ) {
    const dc = slug === "dc-offset";
    const wave = (x) => {
      const sine = Math.sin(((x - 50) * Math.PI) / 70);
      if (slug === "clipping") return 135 - Math.max(-1, Math.min(1, 1.5 * sine)) * 65;
      if (slug === "dropouts" && x >= 302 && x <= 356) return 135;
      if (slug === "impulses" && x === 330) return 40;
      return 135 - (dc ? 25 : 0) - 45 * sine;
    };
    drawing =
      label(50, 28, fr ? "Amplitude" : "Amplitude") +
      line(50, 135, 610, 135) +
      label(26, 140, "0") +
      path(wave) +
      label(530, 238, fr ? "Temps →" : "Time →");
    if (slug === "clipping")
      drawing +=
        line(50, 70, 610, 70, "diagram-limit") +
        label(460, 60, fr ? "Pleine échelle" : "Full scale");
    if (dc)
      drawing +=
        line(50, 110, 610, 110, "diagram-limit") +
        label(425, 66, fr ? "Moyenne décalée" : "Shifted mean");
    if (slug === "true-peak") {
      drawing =
        label(
          50,
          28,
          fr ? "Points stockés et onde reconstruite" : "Stored points and reconstructed wave",
        ) +
        line(50, 135, 610, 135) +
        line(50, 63, 610, 63, "diagram-limit") +
        path((x) => 135 - 90 * Math.sin(((x - 50) * Math.PI) / 140));
      for (let x = 85; x < 610; x += 70)
        drawing += `<circle class="diagram-sample" cx="${x}" cy="${135 - 90 * Math.sin(((x - 50) * Math.PI) / 140)}" r="5"/>`;
      drawing += label(390, 52, fr ? "Plafond numérique" : "Digital ceiling");
    }
    caption = fr
      ? "Signal synthétique illustratif. Les dimensions du dessin ne représentent pas les seuils ni les durées du détecteur ; ceux-ci sont précisés dans la méthode ci-dessous."
      : "Illustrative synthetic signal. Drawing dimensions do not represent detector thresholds or durations; these are specified in the method below.";
  } else if (slug === "upscaling") {
    drawing = label(
      50,
      38,
      fr ? "Exemple : 16 bits élargis à 24 bits" : "Example: 16 bits widened to 24 bits",
    );
    for (let i = 0; i < 24; i++)
      drawing += `<rect class="${i < 16 ? "diagram-bit" : "diagram-empty"}" x="${50 + i * 23}" y="85" width="18" height="55"/>`;
    drawing +=
      label(50, 185, fr ? "16 bits de signal" : "16 signal bits") +
      label(415, 185, fr ? "8 bits à zéro" : "8 zero bits");
    caption = fr
      ? "Cas exact d’un élargissement sans bruit ajouté. La recherche de grille traite séparément les faibles résidus ; un résultat estimé est affiché avec ≈."
      : "Exact widening without added noise. Grid detection handles small residuals separately; an estimated result is marked ≈.";
  } else {
    const lattice = slug === "transcoding";
    drawing =
      label(
        50,
        30,
        lattice
          ? fr
            ? "Valeurs regroupées sur une grille"
            : "Values clustered around a grid"
          : fr
            ? "Niveau spectral relatif"
            : "Relative spectral level",
      ) + line(50, 210, 610, 210);
    if (lattice) {
      for (let i = 0; i < 8; i++) {
        const x = 75 + i * 72;
        drawing +=
          line(x, 65, x, 190) +
          `<circle class="diagram-sample" cx="${x - 4}" cy="105" r="4"/><circle class="diagram-sample" cx="${x + 3}" cy="150" r="4"/>`;
      }
    } else
      drawing += path((x) =>
        slug === "dsd-heritage"
          ? x < 280
            ? 90 + (x - 50) / 5
            : x < 400
              ? 170
              : 170 - (x - 400) / 3
          : x < 345
            ? 75 + (x - 50) / 7 + 6 * Math.sin(x * 0.07)
            : 193 + 3 * Math.sin(x * 0.09),
      );
    drawing += label(
      425,
      244,
      lattice ? (fr ? "Coefficients →" : "Coefficients →") : fr ? "Fréquence →" : "Frequency →",
    );
    caption = lattice
      ? fr
        ? "Schéma de principe d’une grille de quantification. Une proximité statistique ne constitue pas une preuve d’origine."
        : "Conceptual quantization grid. Statistical proximity does not prove origin."
      : fr
        ? "Forme spectrale illustrative, sans échelle de mesure. Une forme identique peut être produite par plusieurs procédés ; consulter les limites de l’analyse."
        : "Illustrative spectrum without a measurement scale. Different processes can produce the same shape; see the analysis limits.";
  }
  return `<figure class="guide-diagram"><svg viewBox="0 0 660 265" role="img" aria-labelledby="diagram-title diagram-desc"><title id="diagram-title">${fr ? "Comprendre la mesure" : "Understanding the measurement"}</title><desc id="diagram-desc">${esc(caption)}</desc>${drawing}</svg><figcaption>${esc(caption)}</figcaption></figure>`;
}
