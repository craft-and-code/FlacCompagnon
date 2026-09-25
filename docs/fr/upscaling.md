# Détection de la profondeur entière

L’application fournit une indication binaire `Upscaled`. Un résultat négatif signifie qu’aucune des signatures ci-dessous n’a été établie, sans certifier l’historique d’enregistrement. Les contrôles indisponibles sont expliqués dans les détails ; les échecs de lecture restent des erreurs explicites.

## Bits inutilisés exacts

Chaque échantillon entier décodé, dans tous les canaux et jusqu’à la fin, contribue à un OU binaire. Les bits de poids faible toujours nuls identifient exactement une précision inutilisée. Élargir du PCM 16 bits à 24 bits par multiplication par 256 laisse ainsi huit bits bas à zéro. Les valeurs négatives sont traitées en complément à deux.

La convention historique du silence est conservée : un signal entièrement nul affiche un bit effectif et est signalé si la profondeur déclarée est supérieure. Ce bit est une convention, pas la preuve d’un enregistrement sur un bit. Un flux vide ou cassé est une erreur de décodage.

Les valeurs doivent rester entières, y compris en PCM 32 bits. Les flottants alimentent le spectre mais ne permettent pas de reconstruire les bits faibles entiers. Une source flottante n’a donc pas de profondeur entière mesurée.

## Grille sous un faible bruit d’export

Un export peut ajouter un petit résidu à un signal élargi. Le OU binaire détecte alors correctement tous les bits occupés, mais masque l’ancienne grille de précision. Pour un pas candidat `S = 2^(declared_bits - candidate_bits)`, un second contrôle calcule en arithmétique entière la distance au multiple de S le plus proche.

Les conditions sont conservatrices :

- profondeur source candidate d’au moins 8 bits, écart d’au moins 6 bits ;
- chaque échantillon doit rester à `S/16` ou moins de la grille ; un seul dépassement, même dans le dernier paquet, rejette définitivement le candidat ;
- au moins trois fenêtres disjointes de 4096 échantillons par canal non nul doivent chacune couvrir 64 pas de grille et peupler au moins 32 des 64 résidus de codes grossiers ;
- chaque canal est traité séparément et le fichier retient la plus grande profondeur nécessaire ; un canal natif n’est pas masqué par plusieurs canaux élargis ou silencieux ;
- tous les échantillons restent contrôlés même après l’obtention d’assez de fenêtres riches.

Les signaux à peu de niveaux, un DC avec un bruit minuscule, les signaux très faibles, impulsions rares et silence ne fournissent pas cette preuve de grille. Les seuils sont des choix techniques, pas des niveaux de confiance publiés. Pour une grille 16 bits dans du 24 bits, les résidus acceptés occupent 33 entiers sur 256 : ce ratio n’est pas un taux de faux positifs, la musique n’étant pas un bruit uniforme indépendant.

L’export Audacity étudié contient 16 558 080 échantillons. Une mesure PCM indépendante avec FFmpeg trouve un résidu de −10 à +10 LSB de destination, de RMS 2,050271, autour d’une grille 16 bits couvrant 64 972 codes. Les 24 bits sont occupés. 142 échantillons dépassent ±8, d’où le choix de la borne ±16 ; les tests incluent aussi des contre-exemples, sans se limiter à ce fichier.

## Résultat et rapports

L’affichage `≈16-bit` signale l’estimation. Le champ `bit_depth_evidence.stored_bits` conserve les bits exactement occupés, et les détails expliquent le résidu. Le JSON retient les deux valeurs ; le CSV garde `bit_depth_method` et `stored_bits` en complément de `real_bit_depth`. Les anciens rapports sans ce champ restent lisibles.

## Limites et approches écartées

Un signal synthétique sur une grille, ou une musique de grille 16 bits mélangée à un vrai signal plus faible que le résidu accepté, peut être impossible à distinguer d’un export avec dither. Le résultat est donc une estimation, pas une affirmation sur la nature du résidu. Bruit plus fort, gain, rééchantillonnage ou traitement peuvent effacer la grille.

Il ne faut pas déduire une profondeur source d’un plancher à −98 dBFS. La formule `6,02×N + 1,76 dB` concerne un quantificateur idéal avec sinus pleine échelle dans des conditions particulières. Bruit acoustique, gain, dither, bande FFT et normalisation empêchent un seuil universel. Un enregistrement natif 24 bits bruité n’est pas forcément élargi ; un plancher faible ne certifie pas non plus un master natif. Un arrêt après 100 000 échantillons serait insuffisant : une introduction élargie ne décrit pas la suite du fichier.

## Sources et reproductibilité

La validation consignée le 23 septembre 2026 comprend 296 tests Rust, dont huit tests transversaux d’upscaling, les contrôles TypeScript, la compilation frontend, 12 tests de régression frontend et Clippy sur toutes les cibles.

| Fichier fourni                        | Déclaré | Effectif                 | Bits occupés | Upscaled |
| ------------------------------------- | ------- | ------------------------ | ------------ | -------- |
| `test_upscaled.flac`                  | 24      | environ 16               | 24           | oui      |
| `11 You Can Make Your Own Music.flac` | 16      | 1, convention du silence | 1            | oui      |

Les 13 pistes 24 bits / 96 kHz de l’album No More Tears fourni ont également été vérifiées dans le pipeline complet : 24 bits effectifs, Clean, aucune détection et aucune erreur de lecture. Ces cas et le corpus synthétique ne constituent pas un taux d’erreur universel. Les résultats d’une ancienne application ou d’un rapport importé doivent être recalculés avec la nouvelle version ; les états de détection devenus incompatibles nécessitent une nouvelle analyse.

- [RFC 9639, bits inutilisés du FLAC](https://www.rfc-editor.org/rfc/rfc9639.html#section-9.2.2) : bourrage exact et restauration des bits, prise en charge de 4–32 bits.
- [Dither dans Audacity](https://manual.audacityteam.org/man/dither.html) : traitement flottant et export ; la valeur illustrative ±3 ne borne pas l’export étudié, mesuré à ±10.
- [Analog Devices, bruit des convertisseurs](https://www.analog.com/en/resources/technical-articles/selecting-the-best-data-converter-for-a-given-noise-budget-part-3.html) : hypothèses du SNR et de l’ENOB, distincts de la largeur de stockage.
- [Lipshitz, Wannamaker et Vanderkooy, Quantization and Dither (1992)](https://hajim.rochester.edu/ece/sites/zduan/teaching/ece472/reading/Lipshitz_1992.pdf) : propriétés du dither et hypothèses statistiques.

L’implémentation est dérivée de l’arithmétique entière et de ces principes. Pour lancer le corpus synthétique puis une sonde en lecture seule :

```sh
cargo test -p flaccompagnon-core --offline
cargo run --release -p flaccompagnon-core --example bitdepth -- /path/to/track.flac
```
