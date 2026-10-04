# Coupures suspectes

La colonne Dropouts compte les courts intervalles de zéros exacts entourés de PCM actif. Cette forme peut indiquer une perte d’audio, mais aussi un montage, une coupure volontaire ou du matériel synthétique.

## Signal recherché

Chaque canal est testé séparément pour une suite de zéros de 2 à 250 ms, bornes incluses. Le minimum est arrondi au nombre supérieur d’échantillons, le maximum au nombre inférieur. Chaque bord doit présenter un saut d’au moins 0,02 de la pleine échelle. Les contextes réels avant et après contiennent `ceil(sample_rate / 200)` échantillons, soit au moins 5 ms, et leur RMS doit atteindre −60 dBFS. Le rapport des puissances moyennes après et avant doit rester entre `1/16` et `16`, bornes incluses (environ ±12,04 dB), avec au moins 90 % d’échantillons non nuls dans le contexte repris.

Les silences de début et de fin et les contextes trop courts sont exclus. Un fondu doux dont les sauts restent sous le seuil est rejeté, mais un fondu assez rapide et fort autour d’un silence peut être compté. Aucun zéro artificiel n’est ajouté pour fabriquer le contexte.

## Résultat et limites

L’application affiche un nombre ou un tiret ; le survol donne canal, début et durée. Au plus 32 positions sont conservées, mais le compteur reste complet. Des trous simultanés à gauche et à droite sont deux événements de canal.

La mesure prend en charge 1–32 canaux à 8–768 kHz. Les données invalides, flux très courts et DSD n’ont pas de résultat. Un trou rempli de dither, de décalage continu ou de bruit n’est pas exactement nul et échappe au test. Les passages manquants plus longs, fondus progressifs et coupures masquées par le décodage avec perte peuvent aussi être manqués. Les flottants finis hors de ±1 sont testés sans écrêtage ; le seuil absolu des sauts dépend donc du gain.

Ce test examine l’onde décodée, sans observer les interruptions de lecture ni l’historique de transport de l’enregistrement. Des paquets perdus ne deviennent pas forcément des zéros : [Opus prévoit une dissimulation des pertes](https://www.rfc-editor.org/rfc/rfc6716.html#section-4.4), par exemple. Le compteur seul ne distingue pas un défaut d’une coupure volontaire et n’établit pas leur audibilité.

## Précision, coût et alternatives

L’énergie du contexte glissant utilise une [somme compensée](https://doi.org/10.1002/zamm.19740540106), avec ajouts et retraits séparés. Elle conserve l’énergie de niveau ordinaire après qu’un flottant fini extrême a quitté la fenêtre. Le travail est linéaire dans le nombre d’échantillons ; l’historique contient `ceil(sample_rate / 200)` valeurs en double précision par canal et ne grandit pas avec la durée du fichier. Les buffers combinés des deux détecteurs de discontinuités sont décrits dans [Impulses](impulses.md).

Un détecteur tolérant au bruit demanderait une autre définition et un plancher de bruit mesuré ; il pourrait aussi classer des passages musicaux faibles comme des trous. Les méthodes prédictives évoquées dans [Impulses](impulses.md) permettraient de tester certaines discontinuités sans exiger des zéros exacts, mais ajouteraient des hypothèses et un coût. Aucune de ces alternatives n’est implémentée. Le réglage des seuils demande des enregistrements annotés avec coupures volontaires, fondus, passages faibles et défauts réels, ainsi que des écoutes ; les fixtures synthétiques vérifient les bornes implémentées, pas un taux de détection universel.

## Vérification et exemple

```sh
cargo test -p flaccompagnon-core analysis::discontinuities
cargo test -p flaccompagnon-core --test discontinuities
node --test tests/analysis-cells.test.mjs tests/search.test.mjs
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*317*t)|if(between(n\,26400\,27839)\,0\,0.1*sin(2*PI*317*t)):s=48000:d=1' -c:a pcm_s16le dropout-right.wav
```

Les tests couvrent les deux limites de durée à plusieurs fréquences, les canaux indépendants, les bords de contexte, les limites de rapport RMS et de pourcentage non nul, les trous bruités, les fondus rapides volontaires et les grands flottants décodés depuis WAV.

Attendez un dropout sur le canal 2 vers 0,550 s, de 30 ms. Dans Audacity, zoomez à cette position pour voir l’intervalle exactement nul.
