# Impulsions suspectes

La colonne Impulses compte les candidats à une impulsion courte et isolée dans le PCM décodé. Elle conserve des positions à écouter. Il s’agit d’un indice de restauration : percussions, montages, impulsions volontaires et artefacts de codecs peuvent satisfaire la même règle.

## Signal recherché

Chaque canal est examiné séparément. Le candidat dure au plus 0,5 ms, avec des bords d’entrée et de sortie qui se compensent largement. Chaque bord doit atteindre 0,05 de la pleine échelle et au moins huit fois le RMS des différences premières dans les 5 ms réelles avant et après. L’écart de compensation des bords ne doit pas dépasser 25 % du plus grand bord.

Après un candidat, les bords dans la milliseconde suivante sont regroupés pour éviter de multiplier les positions pour une même perturbation courte.

## Résultat et limites

L’application affiche seulement `0`, un entier positif ou un tiret. Le survol donne temps, canal et durée. Les 32 premières positions, triées par temps et canal, sont retenues dans les rapports ; le compteur continue au-delà. Deux candidats simultanés dans deux canaux comptent séparément.

La mesure accepte 1–32 canaux à 8–768 kHz. Flux invalides, durées inférieures à environ 11 ms et DSD n’ont pas de résultat : le filtrage du décodage DSD modifie précisément la forme recherchée. Aucun silence artificiel n’est ajouté aux bords ; un événement sans contexte réel n’est pas évalué.

Des clics faibles, larges, masqués ou filtrés peuvent être manqués. Un zéro ne garantit pas l’absence de discontinuité audible. Inspectez les positions dans un éditeur avant toute réparation.

## Vérification et exemple

```sh
cargo test -p flaccompagnon-core analysis::discontinuities
cargo test -p flaccompagnon-core --test discontinuities
node --test tests/analysis-cells.test.mjs tests/search.test.mjs
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*317*t)+if(eq(n\,12000)\,0.6\,0)|0.1*sin(2*PI*317*t):s=48000:d=1' -c:a pcm_s16le click-left.wav
```

Attendez une impulsion sur le canal 1 vers 0,250 s, d’environ 0,021 ms. Dans Audacity, zoomez jusqu’à l’échantillon ajouté.
