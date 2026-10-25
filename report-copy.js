import english from "./vendor/phemios-analysis/english.js";

const words = {
  en: {
    drop: "Drop your JSON report here",
    choose: "or click to choose a file · JSON · up to 16 MiB",
    privacy: "Your file stays in your browser. No file, path or result is sent to a server.",
    reading: "Reading the report…",
    clear: "Clear the report",
    input: "FlacCompagnon JSON report",
  },
  fr: {
    drop: "Déposez votre rapport JSON ici",
    choose: "ou cliquez pour choisir un fichier · JSON · jusqu’à 16 Mio",
    privacy:
      "Votre fichier reste dans votre navigateur. Aucun fichier, chemin ou résultat n’est envoyé à un serveur.",
    reading: "Lecture du rapport…",
    clear: "Effacer le rapport",
    input: "Rapport JSON FlacCompagnon",
  },
};
const errors = {
  "Choose one JSON report at a time.": "Choisissez un seul rapport JSON à la fois.",
  "Choose a JSON report exported by FlacCompagnon.":
    "Choisissez un rapport JSON exporté par FlacCompagnon.",
  "The report exceeds the 16 MiB limit.": "Le rapport dépasse la limite de 16 Mio.",
  "Unable to read this report.": "Impossible de lire ce rapport.",
};
const importedMatch =
  "Conforme dans le rapport ; le fichier actuel n’a pas été vérifié par Phémios.";

export function reportCopy(locale) {
  return words[locale === "fr" ? "fr" : "en"];
}

export function reportTranslate(locale) {
  return (message, ...parameters) => {
    const text =
      message === importedMatch
        ? locale === "fr"
          ? "Conforme dans le rapport ; cette page ne revérifie pas le fichier audio actuel."
          : "Matching in the report; this page does not reverify the current audio file."
        : locale === "fr"
          ? (errors[message] ?? message)
          : (english[message] ?? message);
    return text.replace(/\{(\d+)\}/g, (match, index) =>
      index < parameters.length ? String(parameters[index]) : match,
    );
  };
}
