# Spectrogrammes PNG natifs

**Generate spectrograms** crée un PNG par piste dans un dossier `spectrograms/`
à côté des fichiers audio. Les sources restent en lecture seule. Un rendu réussi
remplace l’image précédente de façon atomique ; une erreur ou une annulation la
préserve.

Le calcul, la composition, les textes et l’encodage PNG sont entièrement écrits
en Rust. Aucun processus Python ni moteur graphique externe n’intervient.
**FFmpeg reste nécessaire uniquement au décodage du contenu DSD (`.dsf`/`.dff`)**,
y compris pour les spectrogrammes. Les autres formats pris en charge utilisent
les décodeurs natifs déjà employés pour l’analyse.

## Taille et couleur

Le menu **Spectrograms** propose deux sous-menus, **Size** et **Color** :

- **Standard (1200 × 627)** est le format par défaut. La hauteur est l’arrondi
  de 1200 × 470 / 900.
- **Large (1800 × 940)** conserve la surface détaillée existante.
- **Green**, commun à FlacCompagnon et Aède, est la couleur par défaut.
  **Blue**, **Amber** et **Violet** proposent d’autres dégradés monochromes,
  avec la même échelle de niveaux.

Ces choix sont mémorisés. Le menu change la préférence ; les images sont écrites
uniquement après un clic sur **Generate spectrograms**. Une ancienne préférence
`half` sélectionne désormais Standard ; `full` conserve Large.

Ces dimensions concernent la zone spectrale. Avec le cadre par défaut, le PNG
complet mesure 1448 × 903 ou 2048 × 1216. Sa compression est sans perte.

## Lire l’image

Le temps est horizontal, les fréquences verticales jusqu’à la limite de Nyquist
du signal décodé. La légende à droite représente −120…0 dBFS/bin du sombre au
clair. La couleur décrit un niveau, pas une catégorie de détection ni un canal.
Un gain automatique par morceau ne vient pas déplacer cette correspondance.

La signature de l’application apparaît en haut à droite, le titre et l’artiste
en bas à gauche, les caractéristiques réelles en bas à droite : fréquence
décodée, profondeur disponible et Mono/Stereo/nombre de canaux. Un titre absent
est remplacé par le nom du fichier. Une profondeur inconnue reste inconnue.
Le DSD est explicitement présenté comme une vue de son décodage PCM.

Chaque canal est transformé avant de combiner les puissances ; le contenu en
opposition de phase reste ainsi visible. Le réglage par défaut emploie une FFT
avec fenêtre de Hann et 87,5 % de recouvrement. Les pixels retiennent les crêtes,
y compris les événements brefs. Le pied de page et les métadonnées du PNG
conservent les réglages. Le [contrat du crate](https://github.com/craft-and-code/FlacCompagnon/tree/main/spectrogram#readme) détaille
la normalisation et les limites de résolution temporelle et fréquentielle.

Une limite spectrale peut venir d’un filtre, de l’enregistrement ou d’un encodeur.
Elle ne prouve pas, isolément, une origine avec perte. Le moteur accepte des
repères mesurés de coupure ou de temps ; l’application n’affiche pas de ligne de
coupure supposée automatiquement.

## Réutilisation et vérification

`flaccompagnon-spectrogram` ne dépend ni de Tauri, ni d’une application, ni d’un
décodeur et n’accède pas au système de fichiers. Le module
`flaccompagnon-core::spectrogram` réexporte son API et fournit l’adaptateur de
lecture audio. Aède utilise ce même crate issu du même tag via sa dépendance au
cœur d’analyse. L’application appelante choisit signature, textes, couleurs,
dimensions et annotations.

Le service Rust de l’application peut aussi être exercé directement :

```sh
cargo run -p flaccompagnon-services --example spectrogram -- input.flac output.png green
cargo run -p flaccompagnon-services --example spectrogram -- input.flac output.png blue 1800 940
```

Pour du DSD, cet exemple demande aussi `FLACCOMPAGNON_FFMPEG` avec le chemin du
décodeur. L’application le recherche automatiquement pour le DSD uniquement.

```sh
cargo test -p flaccompagnon-spectrogram
cargo test -p flaccompagnon-core spectrogram
cargo test -p flaccompagnon --lib spectrogram
```
