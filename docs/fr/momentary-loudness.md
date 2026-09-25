# Maximum de loudness momentané (LUFS-M max)

La colonne **LUFS-M max**, après LUFS, indique la fenêtre complète de **400 ms** la plus forte. La cellule affiche seulement le nombre, à une décimale ; l’en-tête porte l’unité. Au survol, l’infobulle fournit le début et la fin de cette fenêtre.

## Calcul

Le calcul réutilise la puissance pondérée K du [LUFS intégré](integrated-loudness.md). À chaque trame décodée, il additionne les dernières `round(sample_rate × 0.4)` puissances et divise par la longueur de fenêtre. Les puissances des canaux mono ou stéréo, de poids unitaires, sont additionnées.

```text
M = −0.691 + 10 × log10(mean K-weighted channel-summed power)
```

La plus grande somme détermine le maximum et sa position ; en cas d’égalité exacte, la première position est conservée. Le début est `(decoded_frames − window_frames) / sample_rate`, avec le compteur de trames pris à la fin de la fenêtre.

Conformément à [EBU Tech 3341, §2.1–2.2](https://tech.ebu.ch/docs/tech/tech3341.pdf), M n’applique aucune porte absolue ou relative. Un son très faible, sous −70 LUFS, peut donc avoir une valeur M alors que le LUFS intégré est indisponible.

Toutes les fenêtres complètes alignées sur les échantillons sont examinées, y compris celles qui se terminent entre deux mises à jour de 100 ms. Aucune fenêtre partielle au début et aucun silence ajouté à la fin ne participent. Le maximum est capturé avant l’ajout des 1,5 secondes internes utilisées pour la LRA.

## Disponibilité et limites

- Mono ou stéréo à 8–768 kHz, y compris le PCM issu du décodage DSD. Le multicanal reste indisponible tant que les positions et poids ne sont pas fiables.
- Moins de 400 ms complets : pas de résultat. Aux fréquences inhabituelles, la longueur est arrondie à un nombre entier de trames.
- Le silence a une puissance nulle et un loudness théorique de −∞. L’application affiche un tiret. Les échantillons invalides, les échecs de décodage et les anciens rapports sans mesure donnent également un tiret.
- Il s’agit du maximum **sur le fichier**, pas du niveau au point de lecture. Ce n’est pas une crête instantanée ni une preuve de clipping.
- La position désigne l’intervalle mesuré, pas l’attaque exacte d’un événement. La pondération K possède un historique de filtrage, initialement nul. Mesurer un extrait exporté séparément peut modifier les résultats aux bords.

## Rapports et affichage

Le JSON conserve `loudness_peaks.momentary` sous la forme `{ "lufs": nombre, "start_secs": nombre }`, ou `null`. Le champ externe `loudness_peaks` est optionnel pour lire les anciens rapports. Le CSV ajoute `max_momentary_lufs` et `momentary_max_start_s` après `integrated_lufs`. Les valeurs gardent leur précision stockée ; une absence reste vide et un zéro mesuré reste zéro.

L’ordre par défaut est **LUFS → LUFS-M max → LUFS-S max → LRA**. Les préférences existantes insèrent les nouvelles colonnes après LUFS à leur première apparition, puis conservent les déplacements et choix de visibilité de l’utilisateur. Le tri utilise les valeurs non arrondies et place les absences en dernier dans les deux sens.

`core/src/analysis/loudness_peaks.rs` gère les maxima et leurs positions ; `loudness.rs` fournit la puissance pondérée et les fenêtres partagées. Le suivi ajoute un état de taille fixe, sans nouveau filtrage ni nouveau tampon d’échantillons.

## Vérification

```sh
cargo test -p flaccompagnon-core loudness_peaks
cargo test -p flaccompagnon-core --test loudness_reference -- --ignored --nocapture
node --test tests/analysis-cells.test.mjs tests/search.test.mjs
```

Les tests séparés du code reproduisent notamment les 20 décalages du cas 13 d’EBU Tech 3341 : ils révèlent les maxima manqués par une simple grille de 100 ms. D’autres cas vérifient les faibles niveaux non filtrés par une porte, la durée minimale, les dernières fenêtres, les erreurs et l’exclusion de la fin artificielle de LRA. Un test WAV vérifie décodage, positions, CSV/JSON et anciens rapports.

FFmpeg journalise M/S à 10 Hz : pour un bref transitoire, le maximum de ses lignes peut être inférieur au maximum recherché à chaque trame. Les comparaisons automatiques utilisent des niveaux soutenus, à plusieurs fréquences d’échantillonnage.

## Exemple manuel

Créez une salve stéréo de 1 kHz, de 400 ms, décalée de la grille de 100 ms :

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=if(between(t\,2.04\,2.44)\,0.0707945784\,0)*sin(2*PI*1000*t)|if(between(t\,2.04\,2.44)\,0.0707945784\,0)*sin(2*PI*1000*t):s=48000:d=6' -c:a pcm_s24le momentary.wav
ffmpeg -hide_banner -nostats -i momentary.wav -af ebur128 -f null -
```

Attendez **LUFS-M max ≈ −23,0**, avec une fenêtre proche de 2,04–2,44 s, et **LUFS-S max ≈ −31,8** : `−23 + 10 log10(0,4 / 3) ≈ −31,75`. Dans Audacity, utilisez un sinus stéréo d’amplitude linéaire `0.0707945784`, gardez 400 ms et ajoutez du silence autour.
