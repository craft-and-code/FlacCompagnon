// Shared by the homepage, guide index and article navigation.
export const groups = [
  { id: "authenticity", fr: "Authenticité & spectre", en: "Authenticity & spectrum" },
  { id: "stereo", fr: "Stéréo & phase", en: "Stereo & phase" },
  { id: "levels", fr: "Niveaux & dynamique", en: "Levels & dynamics" },
  { id: "loudness", fr: "Loudness", en: "Loudness" },
  { id: "integrity", fr: "Intégrité & discontinuités", en: "Integrity & discontinuities" },
  { id: "dsd", fr: "Sources DSD", en: "DSD sources" },
];

const entries = [
  [
    "upscaling",
    "authenticity",
    "Profondeur réelle",
    "Real bit depth",
    "Bits inutilisés et grille de quantification sous un faible résidu.",
    "Unused bits and quantization grids beneath small residuals.",
  ],
  [
    "upsampling",
    "authenticity",
    "Upsampling",
    "Upsampling",
    "Bande haute durablement vide dans un conteneur haute résolution.",
    "Persistent empty high band in a high-rate container.",
  ],
  [
    "transcoding",
    "authenticity",
    "Transcodage AAC / MP3",
    "AAC / MP3 transcoding",
    "Recherche statistique de grilles laissées par un codec avec perte.",
    "Statistical search for quantization grids left by lossy codecs.",
  ],
  [
    "spectral-cutoff",
    "authenticity",
    "Coupure spectrale",
    "Spectral cutoff",
    "Fréquence limite, chute et niveau de la bande supérieure.",
    "Content boundary, cliff and upper-band level.",
  ],
  [
    "fake-stereo",
    "stereo",
    "Dual mono",
    "Dual mono",
    "Deux canaux identiques ou une différence stéréo négligeable.",
    "Identical channels or a negligible stereo difference.",
  ],
  [
    "stereo-polarity",
    "stereo",
    "Polarité globale",
    "Global polarity",
    "Corrélation gauche/droite et forte opposition sur la piste entière.",
    "Left/right correlation and strong whole-track opposition.",
  ],
  [
    "local-phase",
    "stereo",
    "Phase locale & par bande",
    "Local & band phase",
    "Oppositions brèves ou masquées dans quatre bandes de fréquences.",
    "Brief or hidden opposition across four frequency bands.",
  ],
  [
    "stereo-balance",
    "stereo",
    "Balance stéréo",
    "Stereo balance",
    "Écart de niveau RMS entre les canaux gauche et droit.",
    "RMS level difference between left and right channels.",
  ],
  [
    "high-frequency-stereo",
    "stereo",
    "HF Stereo",
    "HF Stereo",
    "Rétrécissement stéréo dans les aigus. Indice expérimental.",
    "Stereo narrowing in the high band. Experimental cue.",
  ],
  [
    "dc-offset",
    "levels",
    "Décalage continu",
    "DC offset",
    "Déplacement moyen par rapport à zéro, mesuré par canal.",
    "Mean displacement from zero, measured per channel.",
  ],
  [
    "clipping",
    "levels",
    "Clipping",
    "Clipping",
    "Suites d’échantillons à pleine échelle ou très proches.",
    "Runs of samples at or very near full scale.",
  ],
  [
    "true-peak",
    "levels",
    "True peak",
    "True peak",
    "Estimation des crêtes entre les échantillons, en dBTP.",
    "Estimated inter-sample peaks, in dBTP.",
  ],
  [
    "dynamic-range",
    "levels",
    "Dynamique (DR)",
    "Dynamics (DR)",
    "Écart entre la crête et le RMS des passages forts.",
    "Gap between sample peak and loud-passage RMS.",
  ],
  [
    "integrated-loudness",
    "loudness",
    "LUFS intégré",
    "Integrated LUFS",
    "Loudness du programme, avec pondération K et portes de mesure.",
    "Programme loudness with K-weighting and measurement gates.",
  ],
  [
    "momentary-loudness",
    "loudness",
    "LUFS-M max",
    "LUFS-M max",
    "Fenêtre de 400 ms la plus forte et sa position.",
    "Loudest 400 ms window and its location.",
  ],
  [
    "short-term-loudness",
    "loudness",
    "LUFS-S max",
    "LUFS-S max",
    "Fenêtre de 3 secondes la plus forte et sa position.",
    "Loudest 3-second window and its location.",
  ],
  [
    "loudness-range",
    "loudness",
    "Loudness Range (LRA)",
    "Loudness Range (LRA)",
    "Variation du loudness à court terme, exprimée en LU.",
    "Variation in short-term loudness, expressed in LU.",
  ],
  [
    "impulses",
    "integrity",
    "Impulsions",
    "Impulses",
    "Impulsions isolées suspectes, avec positions à écouter.",
    "Suspected isolated pulses, with locations to audition.",
  ],
  [
    "dropouts",
    "integrity",
    "Dropouts",
    "Dropouts",
    "Courtes coupures à zéro exact au milieu d’un signal actif.",
    "Short exact-zero gaps within active audio.",
  ],
  [
    "flac-md5",
    "integrity",
    "Signature MD5 FLAC",
    "FLAC MD5 signature",
    "Comparaison du PCM décodé à la signature intégrée au FLAC.",
    "Decoded PCM compared with the signature stored in FLAC.",
  ],
  [
    "fingerprints",
    "integrity",
    "Empreintes MD5 & CRC32",
    "MD5 & CRC32 fingerprints",
    "Identification des octets du fichier, même après déplacement.",
    "File-byte identification, including after a move.",
  ],
  [
    "dsd-pcm-source",
    "dsd",
    "Source PCM dans un DSD",
    "PCM source in DSD",
    "Recherche de frontières compatibles avec du PCM 44,1 / 48 kHz.",
    "Search for boundaries consistent with 44.1 / 48 kHz PCM.",
  ],
  [
    "dsd-heritage",
    "dsd",
    "Héritage DSD dans le PCM",
    "DSD heritage in PCM",
    "Vallée et remontée ultrasonique compatibles avec un transfert DSD.",
    "Spectral valley and ultrasonic rise consistent with a DSD transfer.",
  ],
];

export const analyses = entries.map(([slug, group, fr, en, frDescription, enDescription]) => ({
  slug,
  group,
  title: { fr, en },
  description: { fr: frDescription, en: enDescription },
}));

export const cli = {
  slug: "cli",
  group: "tools",
  title: { fr: "Utiliser la CLI", en: "Using the CLI" },
  description: {
    fr: "Installation, options, exemples, rapports JSON et intégration avec Aède.",
    en: "Installation, options, examples, JSON reports and Aède integration.",
  },
};
export const userGuide = {
  slug: "user-guide",
  group: "guide",
  title: { fr: "Guide de l’utilisateur", en: "User guide" },
  description: {
    fr: "Analyser, parcourir, modifier les tags, convertir et écouter vos fichiers.",
    en: "Analyze, browse, edit tags, convert and listen to your files.",
  },
};
export const pages = [userGuide, cli, ...analyses];
export const baseUrl = "https://craft-and-code.github.io/FlacCompagnon/";
export const repository = "https://github.com/craft-and-code/FlacCompagnon";
export const rustdoc = `${baseUrl}doc/`;
export const aede = "https://craft-and-code.github.io/aede/";
export const externalDocumentationPrefixes = [aede, rustdoc];
export const escapeHtml = (value) =>
  String(value).replace(
    /[&<>"']/g,
    (c) =>
      ({
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        '"': "&quot;",
        "'": "&#39;",
      })[c],
  );
