# Décalage continu

La colonne `DC (%)` mesure le déplacement moyen de l’audio décodé par rapport à zéro. Elle affiche la plus grande moyenne absolue parmi les canaux, en pourcentage de la pleine échelle, à trois décimales. Le survol fournit chaque moyenne signée dans l’ordre des canaux décodés. Des moyennes de +1 % et −2 % donnent ainsi `2.000`.

## Calcul

Tous les échantillons de chaque canal contribuent :

```text
mean[channel] = sum(samples[channel]) / frame_count
max_abs       = max(abs(mean[channel]))
display       = 100 × max_abs
```

L’amplitude est normalisée : `+1.0` correspond à la pleine échelle positive, `0.01` à 1 %. Silence et dernière trame sont inclus, sans pondération fréquentielle, porte, rééchantillonnage, filtre ni durée minimale. Chaque canal dispose d’une somme compensée en double précision pour préserver les petites contributions quand de grandes valeurs opposées se compensent. Les décalages opposés de plusieurs canaux ne s’annulent donc pas dans un mélange mono.

Cette définition par la moyenne arithmétique est décrite dans le [manuel Audacity](https://manual.audacityteam.org/man/dc_offset.html) et la [documentation FFmpeg](https://ffmpeg.org/ffmpeg-filters.html#dynaudnorm). Un déplacement persistant réduit la marge disponible d’un côté de l’onde.

## Disponibilité et limites

- De 1 à 32 canaux décodés, indépendamment de la fréquence d’échantillonnage.
- Le silence numérique vaut zéro ; un flux vide n’a pas de valeur. Des échantillons constants non nuls sont mesurés même sans oscillation audible.
- Une trame mal formée ou un échantillon non fini invalide toute la mesure. Échec, décodage absent et ancien rapport donnent un tiret.
- Pour le DSD, la valeur porte sur le PCM décodé par FFmpeg et peut dépendre de ses filtres. L’analyse limitée à l’en-tête n’a pas de mesure DC.
- Un extrait court, des périodes graves incomplètes ou un montage asymétrique peuvent donner une moyenne non nulle. Des crêtes positives et négatives différentes ne suffisent pas à établir un décalage moyen.
- La moyenne globale ne localise pas les variations : des décalages temporels opposés peuvent se compenser et du silence peut diluer un décalage local.
- `0.000` est un arrondi, pas une preuve de zéro exact. L’affichage supprime le zéro négatif après arrondi.
- Les flottants hors de ±1 sont mesurés sans écrêtage ; le pourcentage peut dépasser 100 %.

Cette mesure descriptive n’a pas de seuil automatique d’alerte. Elle ne prouve ni panne, ni audibilité, ni provenance, et ne modifie pas les verdicts d’authenticité ou le fichier.

## Rapports

Le JSON conserve `dc_offset.channel_means` et `dc_offset.max_abs` en amplitude normalisée, sans arrondi d’affichage. Les anciens rapports restent lisibles et nécessitent une nouvelle analyse pour obtenir la mesure.

Le CSV exporte `dc_offset_max_abs` et `dc_offset_channel_means` également en amplitude normalisée, **pas en pourcentage**. Les moyennes sont séparées par des points-virgules, par exemple `0.01;-0.02`. Les absences restent vides et un zéro mesuré reste `0`.

## Vérification

```sh
cargo test -p flaccompagnon-core dc_offset
cargo test -p flaccompagnon-core --test dc_offset -- --ignored
node --test tests/analysis-cells.test.mjs tests/search.test.mjs
```

Les tests utilisent des décalages exacts entiers ou en fractions binaires. Ils couvrent canaux indépendants, silence, ondes asymétriques de moyenne nulle, périodes partielles, erreurs finales, grands flottants, WAV multicanaux et unités CSV/JSON. Le test facultatif compare les moyennes `astats` de FFmpeg et un FLAC encodé indépendamment, avec la tolérance de son affichage à six décimales.

## Exemple manuel

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.2*sin(2*PI*1000*t)+0.01|0.2*sin(2*PI*1000*t)-0.02:s=48000:d=5' -c:a pcm_f32le dc-offset.wav
ffmpeg -hide_banner -nostats -i dc-offset.wav -af astats=reset=0 -f null -
```

La colonne doit indiquer `2.000`, avec `Ch 1: +1.000%` et `Ch 2: -2.000%` au survol. FFmpeg doit donner environ `0.010000` et `-0.020000` dans les résultats **par canal**, pas dans son agrégat global. Voir [astats](https://ffmpeg.org/ffmpeg-filters.html#astats).

Retirez les termes `+0.01` et `-0.02` et changez le nom de sortie pour créer un témoin centré, attendu à `0.000`. Dans Audacity, la suppression du décalage continu sur une copie doit également ramener la mesure près de zéro.
