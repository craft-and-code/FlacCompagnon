# Impulsions suspectes

La colonne Impulses compte les candidats à une impulsion courte et isolée dans le PCM décodé. Elle conserve des positions à écouter. Il s’agit d’un indice de restauration : percussions, montages, impulsions volontaires et artefacts de codecs peuvent satisfaire la même règle.

## Signal recherché

Chaque canal est examiné séparément. Le candidat dure au plus 0,5 ms, avec des bords d’entrée et de sortie qui se compensent largement. Chaque bord doit atteindre 0,05 de la pleine échelle et au moins huit fois le RMS des différences premières dans les 5 ms réelles avant et après. L’écart de compensation des bords ne doit pas dépasser 25 % du plus grand bord.

Ces seuils sont des choix du projet, pas une norme audio ni un modèle d’audibilité. À la fréquence `r`, chaque contexte contient `ceil(r / 200)` différences premières et la largeur maximale vaut `floor(r / 2000)` échantillons. Les limites suivent donc la grille d’origine, sans rééchantillonnage. Les différences d’entrée et de sortie sont exclues du RMS de fond ; la largeur réelle du candidat détermine le contexte droit nécessaire.

Après un candidat, les bords dans la milliseconde suivante sont regroupés pour éviter de multiplier les positions pour une même perturbation courte.

## Résultat et limites

L’application affiche seulement `0`, un entier positif ou un tiret. Le survol donne temps, canal et durée. Les 32 premières positions, triées par temps et canal, sont retenues dans les rapports ; le compteur continue au-delà. Deux candidats simultanés dans deux canaux comptent séparément.

La mesure accepte 1–32 canaux à 8–768 kHz. Les flux invalides, durées inférieures à `2 × ceil(r / 200) + 3` trames (483 trames, soit 10,0625 ms, à 48 kHz) et DSD n’ont pas de résultat : le filtrage du décodage DSD modifie précisément la forme recherchée. Aucun silence artificiel n’est ajouté aux bords ; un événement sans contexte réel n’est pas évalué. Même une impulsion d’un seul échantillon exige ses deux bords et tout le contexte réel.

Des clics faibles, larges, masqués ou filtrés peuvent être manqués. Un zéro ne garantit pas l’absence de discontinuité audible. Inspectez les positions dans un éditeur avant toute réparation.

Inverser la polarité ou ajouter un biais constant conserve les différences premières. Le gain conserve le rapport relatif bord/contexte, mais peut faire passer un événement sous le seuil absolu de 0,05. Les valeurs float finies au-delà de la pleine échelle nominale ne sont pas écrêtées. Des impulsions voisines peuvent augmenter mutuellement leur RMS de contexte et échapper au test. Le regroupement à 1 ms compte des indices à écouter, pas chaque échantillon modifié.

## Précision, coût et alternatives

Les préfixes d’énergie compensés préservent un contexte faible après une valeur float très forte. Ils sont calculés uniquement lorsqu’un bloc contient un bord d’entrée admissible, avec des buffers réutilisés entre les blocs de 10 ms. La mémoire dépend de la fréquence et du nombre de canaux, pas de la durée. Avec les deux contrôles actifs, les principaux buffers numériques occupent environ 50 Kio en stéréo à 48 kHz et moins de 13 Mio à 768 kHz / 32 canaux, hors décodage et allocations annexes. Les parcours et déplacements de buffers sont linéaires en nombre de trames, avec jusqu’à `floor(r / 2000)` sorties testées par bord d’entrée suffisamment fort. Cette borne ne mesure pas le débit de l’application entière.

Les détecteurs médiane/MAD comme [Hampel](https://www.mathworks.com/help/signal/ref/hampel.html) résistent aux valeurs isolées, mais une petite fenêtre peut aussi sélectionner les extrema d’une onde. La [restauration audio par médiane](https://www.dafx.de/paper-archive/2013/papers/06.dafx2013_submission_46.pdf) exige également de distinguer les transitoires. La [prédiction linéaire parcimonieuse](https://ftp.esat.kuleuven.be/pub/stadius/vanwaterschoot/reports/dufera2019.pdf) modélise le contexte tonal mais ajoute ajustement et optimisation itérative. Ce sont des comparateurs possibles, pas des remplacements implémentés.

Une [étude avec tests d’écoute](https://link.springer.com/article/10.1186/s13636-024-00389-9) compare des modèles auditifs et des ondelettes sur des clics perceptibles. Ses scores ne mesurent pas ce détecteur. La [recherche sur le contexte musical](https://archives.ismir.net/ismir2021/paper/000095.pdf) traite aussi les clics intentionnels comme une difficulté de classification. Un corpus musical annoté et des écoutes restent nécessaires avant de choisir un remplacement ou d’annoncer un taux de faux positifs.

## Vérification et exemple

```sh
cargo test -p flaccompagnon-core analysis::discontinuities
cargo test -p flaccompagnon-core --test discontinuities
node --test tests/analysis-cells.test.mjs tests/search.test.mjs
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*317*t)+if(eq(n\,12000)\,0.6\,0)|0.1*sin(2*PI*317*t):s=48000:d=1' -c:a pcm_s16le click-left.wav
```

Attendez une impulsion sur le canal 1 vers 0,250 s, d’environ 0,021 ms. Dans Audacity, zoomez jusqu’à l’échantillon ajouté.

Les fixtures automatiques fixent indépendamment les échantillons injectés, les temps et le canal. Elles couvrent les limites exactes bord/contexte et largeur, polarité/biais, 8–768 kHz, 32 canaux, clips les plus courts, bords du fichier, limites de blocs, préfixes float extrêmes et plafond de 32 positions. Sons purs, ondes carrées, bruit et attaques amorties servent de contrôles sans représenter tous les transitoires musicaux. L’intégration vérifie également le décodage WAV et les analyses sélectionnées.
