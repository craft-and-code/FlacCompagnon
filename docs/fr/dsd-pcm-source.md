# Source PCM dans un fichier DSD

Cette analyse recherche une frontière de Nyquist PCM nette dans le contenu DSF ou DFF décodé. Le DSD natif présente généralement une remontée progressive du bruit sigma-delta au-dessus de l’audible ; un DSD créé depuis du PCM à 44,1 ou 48 kHz peut conserver une coupure nette à la limite de sa source. Un résultat positif alimente la détection DSD `Upsampled`.

## Calcul

FFmpeg décode le DSD en PCM flottant. Le spectre moyen déjà produit par l’analyse en continu sert à comparer les bandes autour des deux frontières :

| Frontière | Bande inférieure | Bande supérieure |
| --------- | ---------------- | ---------------- |
| 22,05 kHz | 20,05–21,85 kHz  | 22,35–24,05 kHz  |
| 24 kHz    | 22,0–23,8 kHz    | 24,3–26,0 kHz    |

La bande inférieure doit contenir une énergie supérieure à −80 dB par rapport à la crête spectrale. La différence entre les niveaux moyens inférieur et supérieur doit atteindre 30 dB. Si les deux frontières satisfont la règle, la plus forte chute est retenue.

Le seuil de 30 dB vient d’une calibration du projet : environ 3 dB pour le signal synthétique natif de référence, contre environ 50 dB après limitation à 44,1 kHz puis conversion delta-sigma.

## Interprétation et limites

Le résultat établit cette signature spectrale compatible avec du PCM, pas toutes les étapes de production. Filtres de conversion, absence d’énergie près de la frontière, mise en forme du bruit, filtrage ultérieur et bruit ajouté peuvent masquer ou imiter une coupure. Des signaux identiques ne permettent pas de distinguer des histoires différentes.

Sans FFmpeg utilisable ou si le décodage échoue, aucune analyse de contenu n’est disponible. Un en-tête DSF/DFF valide fournit encore les informations du conteneur.

## Vérification

```sh
cargo test -p flaccompagnon-core dsd::spectral
cargo test -p flaccompagnon-core pipeline::dsd
```

Utilisez du DSD de provenance connue avec FFmpeg installé. Un simple ton PCM converti en DSD teste le chemin de traitement, mais ne représente pas une référence de DSD natif musical.
