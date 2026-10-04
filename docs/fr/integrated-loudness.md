# Loudness intégré (LUFS)

Le LUFS intégré mesure le loudness d’un programme selon la méthode pondérée K avec portes de mesure de l’ITU-R BS.1770 et de l’EBU R 128. Il sert à comparer des niveaux de programmes complets. Les colonnes [LUFS-M max](momentary-loudness.md) et [LUFS-S max](short-term-loudness.md) décrivent séparément les fenêtres de 400 ms et 3 secondes les plus fortes.

## Calcul

Chaque canal mono ou stéréo traverse les filtres de pondération K : un filtre rehaussant les hautes fréquences, puis un passe-haut. Les coefficients publiés à 48 kHz sont conservés exactement à cette fréquence. Ailleurs, la fréquence des pôles analogiques est déduite des dénominateurs de référence, puis une transformation bilinéaire avec précompensation fréquentielle adapte les filtres. Le numérateur du passe-haut RLB reste la seconde différence unitaire publiée `[1, −2, 1]` ; seuls ses pôles sont adaptés. Cela évite le déplacement du plateau et le changement involontaire de gain RLB produits par un simple rapport de fréquences aux taux faibles. Le décalage −0,691 reste inchangé, sans facteur de calibration empirique.

La mesure forme des blocs complets de 400 ms avec un pas nominal de 100 ms, soit 75 % de recouvrement. Les fenêtres sont arrondies à l’échantillon le plus proche et le pas à l’entier inférieur, ce qui maintient au moins dix mises à jour par seconde à un taux inhabituel comme 11025 Hz. Un bloc final incomplet est écarté. Deux portes sont ensuite appliquées :

| Porte    | Règle                                                                 |
| -------- | --------------------------------------------------------------------- |
| Absolue  | Conserver les blocs au-dessus de −70 LUFS                             |
| Relative | Parmi ces blocs, conserver ceux au-dessus de leur moyenne moins 10 LU |

La moyenne est calculée en puissance, pas en faisant une moyenne arithmétique des valeurs en décibels. Les comparaisons sont strictes : un bloc exactement au seuil est exclu. Le résultat final est `−0,691 + 10 × log10(puissance moyenne après les portes)`. Le décalage −0,691 et les seuils proviennent de BS.1770.

## Disponibilité et interprétation

La mesure accepte le mono et la stéréo entre 8 et 768 kHz, avec un poids unitaire par canal. Elle est absente pour le silence, les durées inférieures à une fenêtre complète de 400 ms, les flux invalides et les configurations multicanales. Ces dernières attendent des positions de haut-parleurs et un rôle LFE fiables pour appliquer les bons poids.

Le mono est mesuré comme un seul canal, sans compensation implicite de lecture dual-mono. Le dupliquer à l’identique en stéréo augmente le loudness d’environ 3,01 LU.

Une valeur plus négative correspond à un programme moins fort. Le LUFS n’est pas une mesure de crête, de clipping, de dynamique DR ou de qualité de source. Un extrait et une piste entière ne constituent pas le même programme : il faut comparer des mesures de même périmètre.

## Vérification et exemple

```sh
cargo test -p flaccompagnon-core analysis::loudness
cargo test -p flaccompagnon-core --test loudness_reference -- --ignored --nocapture
```

Le second test utilise FFmpeg disponible dans le `PATH`. Il compare des signaux composites mono et stéréo à 8, 11,025, 16, 32, 44,1, 48 et 96 kHz avec des tolérances définies.

Les tests unitaires couvrent aussi les cas intégrés 1–5 publiés dans EBU Tech 3341, les frontières des portes, la réponse fréquentielle confrontée à des métadonnées FFmpeg fixes, la stabilité des pôles entre 8 et 768 kHz et le retour à un passage calme après du PCM flottant très fort. Ces vérifications synthétiques ne certifient pas tous les programmes ni tous les taux.

Pour produire un sinus stéréo nominal à −23 LUFS :

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.0707945784*sin(2*PI*1000*t)|0.0707945784*sin(2*PI*1000*t):s=48000:d=60' -c:a pcm_s24le loudness.wav
ffmpeg -i loudness.wav -af ebur128 -f null -
```

Le résultat attendu est environ −23,0 LUFS. La comparaison indépendante avec FFmpeg n’est pas une certification formelle ; l’arrondi et le compte rendu peuvent légèrement différer.

Références : [ITU-R BS.1770-5](https://www.itu.int/dms_pubrec/itu-r/rec/bs/R-REC-BS.1770-5-202311-I!!PDF-E.pdf), [EBU Tech 3341](https://tech.ebu.ch/docs/tech/tech3341.pdf), [filtre ebur128 de FFmpeg](https://ffmpeg.org/ffmpeg-filters.html#ebur128).
