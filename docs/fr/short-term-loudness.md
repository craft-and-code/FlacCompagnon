# Maximum de loudness à court terme (LUFS-S max)

La colonne **LUFS-S max** suit LUFS-M max et conserve la fenêtre complète de **3 secondes** la plus forte. La valeur est affichée à une décimale et le survol donne le début et la fin de la fenêtre.

## Calcul

Le calcul utilise la puissance pondérée K du [LUFS intégré](integrated-loudness.md) et l’historique glissant de trois secondes partagé avec la [LRA](loudness-range.md). À chaque trame décodée après les trois premières secondes :

```text
S = −0.691 + 10 × log10(mean K-weighted channel-summed power over 3 s)
```

Selon [EBU Tech 3341, §2.2](https://tech.ebu.ch/docs/tech/tech3341.pdf), la fenêtre est rectangulaire, sans porte ni lissage supplémentaire d’attaque ou de relâchement. Toutes les fenêtres complètes alignées sur les échantillons sont examinées, pas seulement les instants de collecte de LRA. Le premier maximum est conservé en cas d’égalité exacte.

Le maximum est capturé avant l’ajout de la fin artificielle de LRA. Aucune fenêtre partielle initiale ni aucun échantillon au-delà du programme réel ne contribue. La page [LUFS-M max](momentary-loudness.md) détaille l’implémentation partagée.

## Disponibilité et limites

- Mono ou stéréo à 8–768 kHz, y compris le PCM décodé du DSD. Le multicanal attend des positions de canaux fiables.
- Un fichier de moins de trois secondes n’a pas de maximum S. Exactement trois secondes permettent une fenêtre complète.
- Silence, données invalides, décodage absent ou échoué et anciens rapports : pas de valeur. Un son non nul sous la porte du LUFS intégré reste mesurable.
- L’historique de trois secondes stocke les puissances en flottants 32 bits pour limiter la mémoire. Une puissance extrême non représentable invalide S et LRA ; M peut rester disponible grâce à son historique 64 bits. Une puissance trop petite pour cette représentation ne produit pas de lecture finie.
- Un maximum n’est pas un niveau typique ni une note de qualité. Une attaque brève est diluée sur trois secondes, qui peuvent aussi inclure une pause ou une transition.
- L’historique des filtres affecte les bords : mesurer séparément un extrait peut changer le résultat.
- S max est le plus haut niveau local **sans porte**, en LUFS. LRA est l’étendue d’une distribution de niveaux S **après portes**, en LU.

## Rapports

Le JSON conserve `loudness_peaks.short_term` sous la forme `{ "lufs": nombre, "start_secs": nombre }`, ou `null`, indépendamment de M. Si les deux sont absents, `loudness_peaks` vaut `null`. Les anciens rapports sans ce champ restent lisibles.

Le CSV exporte `max_short_term_lufs` et `short_term_max_start_s` après les champs M et avant `loudness_range_lu`. Les temps partent de la première trame décodée. Les absences restent vides, les nombres gardent leur précision stockée et le tri met les absences en dernier.

## Vérification

```sh
cargo test -p flaccompagnon-core loudness_peaks
cargo test -p flaccompagnon-core --test loudness_reference -- --ignored --nocapture
```

Les tests couvrent les 20 décalages du cas 10 d’EBU Tech 3341, le niveau de référence stéréo, l’écart mono/stéréo d’environ 3,01 LU, les faibles niveaux, la durée exacte de trois secondes, les entrées invalides et l’exclusion de la fin artificielle LRA. Un test WAV vérifie les changements de niveau, positions, rapports et compatibilité. La comparaison facultative avec FFmpeg couvre plusieurs fréquences et les deux configurations.

## Exemple : salve de trois secondes

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=if(between(t\,2.15\,5.15)\,0.0707945784\,0)*sin(2*PI*1000*t)|if(between(t\,2.15\,5.15)\,0.0707945784\,0)*sin(2*PI*1000*t):s=48000:d=8' -c:a pcm_s24le short-term.wav
ffmpeg -hide_banner -nostats -i short-term.wav -af ebur128 -f null -
```

Attendez **LUFS-S max ≈ −23,0**, près de **2,15–5,15 s**, et un LUFS-M max voisin de −23,0. La réponse des filtres peut légèrement déplacer le maximum. Dans Audacity, un sinus stéréo de 1 kHz à l’amplitude `0.0707945784`, durant trois secondes et entouré de silence, produit le même type de test.

## Exemple : fichier trop court

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.0707945784*sin(2*PI*1000*t)|0.0707945784*sin(2*PI*1000*t):s=48000:d=2' -c:a pcm_s24le too-short-for-s.wav
```

Le LUFS intégré et M max doivent être proches de −23,0, tandis que **S max affiche un tiret**. La salve de 400 ms de la page M permet de comparer la dilution d’une attaque dans les deux fenêtres.
