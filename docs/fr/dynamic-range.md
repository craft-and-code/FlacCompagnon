# Estimation de dynamique DR

Le DR estime un facteur de crête : le rapport entre la crête des échantillons et le RMS des passages soutenus les plus forts. Il décrit la dynamique de niveau, indépendamment de l’origine du fichier ou de la normalisation du loudness.

## Calcul

Le décodeur accumule l’énergie par blocs de 131 072 trames, soit environ trois secondes à 44,1 kHz. Un dernier bloc incomplet participe s’il atteint au moins un quart de cette longueur. Les 20 % de blocs les plus forts, arrondis au supérieur avec un minimum d’un bloc, sont moyennés.

```text
DR = 20 × log10(sample peak / RMS of loudest blocks)
```

La longueur est fixe en trames : à 96 kHz, elle correspond à environ 1,37 seconde. La mesure n’utilise donc pas une durée fixe à toutes les fréquences.

## Interprétation

Une valeur plus grande indique un facteur de crête plus élevé dans les passages sélectionnés. L’interface utilise un repère vert à partir de 12 dB et un avertissement sous 8 dB. Ces couleurs ne prouvent ni la supériorité d’un mastering, ni du clipping ou une mauvaise qualité.

Silence, crête nulle ou RMS invalide donnent une absence de mesure. La crête est celle des échantillons, pas le [true peak](true-peak.md).

## Limites

Ce n’est pas le résultat officiel d’une version particulière de DR Meter, ni la LRA EBU. Les tailles de blocs, pondérations, portes et définitions de crête peuvent produire d’autres nombres. Les pistes courtes ont peu de blocs ; un événement isolé fort influence à la fois la crête et la sélection des blocs.

Utilisez le [LUFS intégré](integrated-loudness.md) pour le niveau du programme et la [LRA](loudness-range.md) pour sa variation dans le temps.

## Vérification

```sh
cargo test -p flaccompagnon-core analysis::analyzer
```

Comparez un sinus faible à une copie comportant une brève salve plus forte. Cette salve doit augmenter la crête davantage que le RMS des blocs forts, et donc augmenter le DR. Ce test illustre le calcul défini, sans établir de conformité à un logiciel externe.
