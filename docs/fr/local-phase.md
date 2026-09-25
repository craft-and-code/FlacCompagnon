# Corrélation de phase locale et par bande

Ces mesures révèlent des oppositions diluées par la [polarité globale](stereo-polarity.md) : un court passage inversé ou des aigus opposés sous des graves plus forts et alignés. Elles aident à localiser des passages à écouter en mono, sans établir une origine d’encodage, une polarité absolue ou un défaut involontaire.

## Résultats dans l’application

| Colonne     | Valeur                                                                | Détails au survol                                                                                         |
| ----------- | --------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Local phase | Corrélation minimale des fenêtres large bande admissibles             | Corrélation cumulée, position du minimum, proportion de fenêtres opposées, nombre de fenêtres admissibles |
| Band phase  | Corrélation minimale parmi les fenêtres admissibles des quatre bandes | Les mêmes éléments pour chaque bande                                                                      |

Les valeurs signées sont arrondies à trois décimales et triées numériquement entre −1 et +1. Les absences sont placées en dernier dans les deux sens. Un tiret signifie indisponible, tandis que `0.000` est une mesure arrondie.

- **+1** : canaux proportionnels et de même polarité, même si leurs niveaux diffèrent.
- **0** : absence de corrélation à décalage nul ; cela inclut un quart de période de déphasage sur un sinus, sans signifier que les signaux sont sans relation.
- **−1** : canaux proportionnels et opposés. Des gains inégaux peuvent empêcher l’annulation totale en mono.

Une fenêtre est dite opposée à partir de **−0,5 vers −1**. C’est un seuil de compte rendu du projet, pas une norme, un seuil d’audibilité ou un verdict d’inversion. Aucune nouvelle alerte d’authenticité n’est créée. La polarité globale conserve sa propre méthode et son seuil.

## Résolution temporelle et bandes

Exactement deux canaux sont acceptés, entre **8 et 768 kHz**, y compris le PCM stéréo issu du DSD décodé. Mono et multicanal ne sont pas réduits arbitrairement à deux canaux.

La taille de transformée est `N = next_power_of_two(ceil(sample_rate / 5))`. La fenêtre dure donc au moins 200 ms et moins de 400 ms. Le pas vaut `N / 2`, soit 50 % de recouvrement. À 48 kHz, la fenêtre dure **341,333 ms** et le pas **170,667 ms**. À 44,1 kHz, la fenêtre dure **371,519 ms**. Ces durées réelles sont conservées dans le rapport.

| Plage         | Limites nominales |
| ------------- | ----------------- |
| Large bande   | 20 Hz–20 kHz      |
| Graves        | 20–200 Hz         |
| Médiums       | 200 Hz–2 kHz      |
| Hauts médiums | 2–6 kHz           |
| Aigus         | 6–20 kHz          |

Les limites supérieures sont plafonnées à Nyquist, soit la moitié de la fréquence d’échantillonnage. Le bord inférieur est inclus, le supérieur exclu. Les cases DC et Nyquist sont exclues. Une bande entièrement au-dessus de Nyquist n’a pas de résultat. Chaque case FFT appartient à la plage contenant sa fréquence centrale ; l’espacement vaut `sample_rate / N`.

Seules les fenêtres complètes participent, sans compléter la dernière avec des zéros. Un fichier plus court qu’une fenêtre est indisponible. Le début est compté depuis la première trame décodée, introductions silencieuses comprises. Il désigne la fenêtre, pas le début exact du changement. La fin est `début + window_secs`. À minima égaux à la précision stockée, la première fenêtre est conservée.

## Calcul

Chaque canal utilise une fenêtre de Hann périodique et sa moyenne pondérée :

```text
w[n] = 0.5 − 0.5 cos(2πn/N), n = 0 … N−1
μ = Σ(w[n] × x[n]) / Σw[n]
X[k] = DFT(w[n] × (x[n] − μ))
```

La convention périodique suit la [documentation Hann de MathWorks](https://www.mathworks.com/help/signal/ref/hann.html). Le fenêtrage réduit les fuites aux bords ; retirer la moyenne évite qu’un décalage continu domine la corrélation. Le [DC offset](dc-offset.md) reste mesuré séparément.

Avec les spectres complexes gauche `L[k]` et droit `R[k]`, on additionne les cases de fréquences positives appartenant à chaque plage :

```text
q = 2 / (N × Σw[n]²)
E_L = q × Σ |L[k]|²
E_R = q × Σ |R[k]|²
C   = q × Σ Re(L[k] × conjugate(R[k]))
correlation = C / sqrt(E_L × E_R)
```

Le facteur `q` provient de Parseval, des deux moitiés symétriques du spectre réel et du gain quadratique du fenêtrage. Il rend la porte d’énergie comparable entre tailles de transformée et s’annule dans la corrélation. Le résultat est borné à −1…+1 contre les arrondis numériques.

Une fenêtre est admissible dans une plage si **les deux** énergies dépassent `10⁻⁶`, soit un RMS de bande supérieur à −60 dBFS par canal. Chaque plage a sa propre porte : des graves forts ne rendent pas des aigus silencieux admissibles. Ce seuil de projet écarte les rapports insignifiants dus au silence ou aux faibles fuites ; ce n’est pas une pondération de loudness.

Pour chaque plage, le rapport garde la corrélation calculée sur la **somme des énergies admissibles**, le minimum local et sa position, la fraction de fenêtres à −0,5 ou moins, ainsi que leur nombre. La corrélation cumulée n’est pas la moyenne arithmétique des coefficients.

La fraction concerne des **fenêtres admissibles qui se recouvrent**, pas des échantillons ni des secondes. Elle ne doit pas être présentée comme un pourcentage de piste affectée. Le silence et les passages sur un seul canal sont exclus du dénominateur. Cinq résumés au plus sont conservés ; mémoire et rapport ne grossissent pas avec la durée.

## Limites d’interprétation

- C’est une corrélation à décalage nul, pas un angle en degrés ni une cohérence quadratique. Voir la [discussion MathWorks sur le spectre croisé et la cohérence](https://www.mathworks.com/help/signal/ug/cross-spectrum-and-magnitude-squared-coherence.html).
- Les résultats sont pondérés par l’énergie. Des composantes peuvent compenser leurs corrélations dans une même bande. Quatre résumés ne constituent pas une courbe de phase en fonction de la fréquence.
- Élargissement stéréo, ambiances, délais et microphones espacés peuvent créer une corrélation négative volontaire. Il faut écouter et connaître la source avant de corriger.
- Un événement plus court qu’une fenêtre peut être dilué. Hann atténue aussi l’influence des bords ; la localisation n’est pas précise à l’échantillon.
- Le fenêtrage réduit les fuites sans les supprimer. Un ton près d’une frontière peut contribuer aux deux bandes. Les composantes subsoniques ou ultrasoniques peuvent également fuir dans la mesure.
- Les basses fréquences contiennent peu de cycles par fenêtre. Un transitoire isolé ou un extrait très court est moins représentatif.
- La porte absolue rend la couverture dépendante du gain. Amplifier un fichier très faible peut rendre des bandes admissibles. Une absence ne prouve pas une compatibilité mono parfaite.
- La polarité globale utilise le flux entier, sans cette suppression du DC, ces fenêtres, limites de bande et portes : les coefficients cumulés peuvent différer.
- Une trame invalide ou un échantillon non fini invalide toute la mesure. Les anciens rapports et décodages absents ou échoués sont indisponibles. Les flottants dépassant la pleine échelle sont mesurés sans écrêtage préalable.

## Rapports enregistrés

Le JSON ajoute `local_phase`, optionnel, avec `window_secs`, `hop_secs`, `analyzed_windows`, `broadband` et quatre `bands`. Chaque bande contient ses limites effectives `low_hz`, `high_hz` et un `summary` optionnel. Le résumé contient `correlation`, `minimum_correlation`, `minimum_start_secs`, `opposed_fraction` et `eligible_windows`. Une bande dont `high_hz <= low_hz` est au-dessus de Nyquist et n’a pas de résumé.

Le CSV ajoute `phase_window_s`, `phase_hop_s`, `phase_analyzed_windows`, puis cinq champs par plage :

| Préfixe             | Plage         |
| ------------------- | ------------- |
| `local_phase_`      | Large bande   |
| `phase_20_200_`     | Graves        |
| `phase_200_2000_`   | Médiums       |
| `phase_2000_6000_`  | Hauts médiums |
| `phase_6000_20000_` | Aigus         |

Les suffixes sont `correlation`, `minimum`, `minimum_start_s`, `opposed_fraction` et `eligible_windows`. Les fréquences des noms sont nominales ; utilisez `sample_rate` pour connaître Nyquist. Fractions entre 0 et 1, temps en secondes, absences vides et zéros conservés. Les empreintes restent les deux dernières colonnes. Les anciens JSON se chargent avec `local_phase: null` et nécessitent une nouvelle analyse.

## Vérification automatisée

```sh
cargo test -p flaccompagnon-core local_phase
node --test tests/analysis-cells.test.mjs tests/search.test.mjs
```

Les valeurs attendues des tests proviennent d’identités de phase des sinus et de sommes d’énergies de tons orthogonaux. Ils couvrent 0°, 60°, 90°, 180°, gains différents, aigus opposés sous des graves alignés, opposition localisée, suppression DC, porte RMS, erreurs, fenêtres incomplètes, taux et Nyquist. Le test WAV complet vérifie temps, bandes, CSV, JSON et anciens rapports. Les tests frontend couvrent valeurs signées, tri, absences, infobulles et recherche.

## Exemples avec FFmpeg ou Audacity

`-n` empêche l’écrasement d’un fichier existant. Les signaux restent sous le clipping.

### Opposition brève masquée par la moyenne

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.2*sin(2*PI*1000*t)|if(between(t\,2\,3)\,-1\,1)*0.2*sin(2*PI*1000*t):s=48000:d=6' -c:a pcm_s24le phase-local.wav
```

Attendez **Local phase ≈ −1,000**, une fenêtre minimale dans 2–3 s et une fraction opposée non nulle. Les médiums sont également opposés. La corrélation globale reste proche de +0,667. Dans Audacity, inversez un seul canal entre 2 et 3 secondes d’un ton stéréo ; le mélange mono s’annule dans ce passage.

### Aigus opposés sous des graves alignés

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.2*sin(2*PI*100*t)+0.1*sin(2*PI*9000*t)|0.2*sin(2*PI*100*t)-0.1*sin(2*PI*9000*t):s=48000:d=5' -c:a pcm_s24le phase-bands.wav
```

Attendez **Local phase ≈ +0,600** et **Band phase ≈ −1,000**. Graves proches de +1, aigus proches de −1 avec 100 % de fenêtres opposées, bandes centrales indisponibles. La valeur large bande est `(0,2² − 0,1²) / (0,2² + 0,1²) = 0,6`. En mono, le 9 kHz s’annule et les graves restent.

### La quadrature est un zéro mesuré

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.2*sin(2*PI*1000*t)|0.2*cos(2*PI*1000*t):s=48000:d=5' -c:a pcm_s24le phase-quadrature.wav
```

Les deux colonnes doivent être proches de **0,000**, avec 0 % de fenêtres opposées. Seuls les médiums ont une mesure. Remplacer le cosinus par le même sinus produit +1 ; le remplacer par son opposé produit −1. Ces essais distinguent une vraie corrélation nulle d’une mesure absente.
