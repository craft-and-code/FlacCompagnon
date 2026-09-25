# Coupures suspectes

La colonne Dropouts compte les courts intervalles de zéros exacts entourés de PCM actif. Cette forme peut indiquer une perte d’audio, mais aussi un montage, une coupure volontaire ou du matériel synthétique.

## Signal recherché

Chaque canal est testé séparément pour une suite de zéros de 2 à 250 ms. Chaque bord doit présenter un saut d’au moins 0,02 de la pleine échelle. Le RMS du contexte réel de 5 ms avant et après doit dépasser −60 dBFS. Le niveau repris doit rester à environ 12 dB du niveau précédent, avec au moins 90 % d’échantillons non nuls dans ce contexte repris.

Ces conditions limitent la confusion avec une transition légitime vers le silence. Silences de début et de fin, fondus et contextes trop courts sont exclus, sans ajout de zéros artificiels.

## Résultat et limites

L’application affiche un nombre ou un tiret ; le survol donne canal, début et durée. Au plus 32 positions sont conservées, mais le compteur reste complet. Des trous simultanés à gauche et à droite sont deux événements de canal.

La mesure prend en charge 1–32 canaux à 8–768 kHz. Les données invalides, flux très courts et DSD n’ont pas de résultat. Un trou rempli de dither ou de bruit n’est pas exactement nul et échappe au test. Les passages manquants plus longs, fondus progressifs et coupures masquées par le décodage avec perte peuvent aussi être manqués.

## Vérification et exemple

```sh
cargo test -p flaccompagnon-core analysis::discontinuities
cargo test -p flaccompagnon-core --test discontinuities
node --test tests/analysis-cells.test.mjs tests/search.test.mjs
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*317*t)|if(between(n\,26400\,27839)\,0\,0.1*sin(2*PI*317*t)):s=48000:d=1' -c:a pcm_s16le dropout-right.wav
```

Attendez un dropout sur le canal 2 vers 0,550 s, de 30 ms. Dans Audacity, zoomez à cette position pour voir l’intervalle exactement nul.
