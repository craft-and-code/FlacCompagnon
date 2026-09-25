# Détection d’un transcodage avec perte

`Transcoded` recherche une grille de quantification laissée dans le PCM après décodage d’une source AAC ou MP3 puis stockage sans perte. Le test ne prend pas un spectre sombre ou une coupure nette pour un verdict : ces formes existent aussi dans des enregistrements légitimes.

## Principe

Un codec à transformée quantifie ses coefficients. Recalculer la transformée avec l’alignement de l’encodeur peut montrer que les valeurs restent anormalement proches des pas autorisés. Le détecteur évalue statistiquement cette proximité dans des fenêtres énergétiques sélectionnées. Son score est un indice pour le modèle testé, pas une probabilité que le fichier soit lossy ni une preuve d’origine.

L’AAC utilise des fenêtres longues de 2048 échantillons et courtes de 256. La recherche examine quatre formes de fenêtre et les 1024 alignements possibles ; un seul échantillon de décalage peut effacer l’effet. Le MP3 demande une chaîne hybride : banque polyphasée de 512 coefficients, 32 sous-bandes, MDCT à 18 points et réduction de repliement spectral. Les 576 alignements d’un granule sont examinés.

Les canaux stéréo sont étudiés en gauche/droite et en Mid/Side pour ne pas masquer la signature d’un codage joint stereo. Les fenêtres et bandes proviennent des standards des codecs ; la recherche porte sur les échantillons décodés fournis par FlacCompagnon.

## Résultat et domaine d’application

Le détail inclut le score même en l’absence de verdict, afin de distinguer une recherche terminée sans signature significative d’une recherche inachevée. Le verdict s’applique aux fréquences tabulées prises en charge, actuellement 32, 44,1 et 48 kHz. Aux autres taux, notamment au-dessus de 48 kHz, la recherche peut être indisponible : cela ne signifie pas que la source est sans perte.

Le résumé devient `Flagged` si ce test ou l’une des deux autres détections d’authenticité est positif. `Clean` signifie qu’aucun détecteur terminé n’a levé d’alerte, sans certifier l’histoire du fichier.

## Limites et calibration

Le modèle suppose qu’un signal non compressé se concentre moins fortement sur la grille du codec qu’un transcodage correspondant. Sons tonals, très faibles, traités ou inhabituels peuvent contredire cette hypothèse. Gain, mélange, rééchantillonnage et réencodage peuvent affaiblir ou effacer la signature. Un codec ou réglage non couvert peut être manqué.

La calibration reste préliminaire : un sinus 24 bits sans perte peut encore franchir le seuil AAC. Les motifs d’indisponibilité et le domaine d’application doivent accompagner le score. Ne le convertissez pas en pourcentage de probabilité et ne l’utilisez pas seul pour affirmer une provenance.

## Vérification et exemple

```sh
cargo test -p flaccompagnon-core transcode
cargo test -p flaccompagnon-core analysis::detections
ffmpeg -n -f lavfi -i 'aevalsrc=0.2*sin(2*PI*997*t)+0.1*sin(2*PI*3401*t):s=48000:d=60' -c:a pcm_s24le source.wav
ffmpeg -n -i source.wav -c:a aac -b:a 128k encoded.m4a
ffmpeg -n -i encoded.m4a -c:a flac aac-roundtrip.flac
```

Les tests du core vérifient transformées, grilles, seuils, alignements et classification complète. Le parcours manuel WAV → AAC → FLAC vérifie l’utilisation ; son résultat peut varier avec l’encodeur installé et le signal. Une validation représentative demande aussi de la musique connue et des témoins sans perte.
