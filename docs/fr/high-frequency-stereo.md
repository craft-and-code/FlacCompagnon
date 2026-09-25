# Rétrécissement de la stéréo dans les aigus

La colonne `HF Stereo` compare l’énergie Side à l’énergie Mid dans les hautes fréquences. Elle signale prudemment les programmes larges dans une bande de référence mais durablement plus étroits au-dessus de 6 kHz. Cet indice **expérimental** peut venir d’un codage intensity stereo ou d’un mixage volontaire ; il n’identifie pas un codec et ne prouve pas un défaut audible.

## Principe

```text
Mid  = (Left + Right) / 2
Side = (Left − Right) / 2
```

Un rapport Side/Mid élevé indique une différence stéréo importante. Un rapport plus négatif indique une image plus étroite. Les deux signaux traversent des filtres identiques, avec des sections Butterworth passe-haut et passe-bas d’ordre huit à chaque bord, puis leurs énergies sont mesurées par blocs non recouvrants de 500 ms.

La bande haute se termine à 20 kHz. Lorsque Nyquist est inférieur ou égal à 20 kHz, aucun filtre supérieur n’est nécessaire. Les transitions restent progressives : il s’agit de bandes nominales, pas de découpes spectrales parfaites. Les coefficients suivent le [W3C Audio EQ Cookbook](https://www.w3.org/TR/audio-eq-cookbook/).

| Bande     | Rôle                                                   |
| --------- | ------------------------------------------------------ |
| 1,5–5 kHz | Référence établissant une largeur stéréo significative |
| 6–20 kHz  | Aigus, limités par Nyquist aux taux plus faibles       |

## Seuils et résultat

Un bloc est admissible si le RMS Mid dépasse −60 dBFS dans les deux bandes. Il est considéré rétréci lorsque les trois règles suivantes sont réunies :

- Side/Mid dans les aigus inférieur ou égal à −20 dB ;
- Side/Mid de référence supérieur ou égal à −12 dB ;
- écart d’au moins 12 dB entre les deux rapports.

L’indice au niveau du fichier demande au moins trois blocs admissibles, dont 70 % satisfont ces règles, **et** les mêmes règles sur les rapports calculés à partir des énergies cumulées. Les blocs ne doivent pas forcément être consécutifs : le pourcentage décrit les blocs admissibles, pas une durée continue.

Le nombre affiché est `10 log10(somme énergie Side / somme énergie Mid)` sur les blocs complets admissibles, en dB. Les blocs faibles et le dernier bloc partiel sont exclus. Un Side nul ou extrêmement faible est affiché à un plancher relatif de −120 dB : c’est une convention d’affichage, pas un bruit mesuré. Le survol donne aussi la référence et la fraction de blocs concernés.

Les seuils sont des choix heuristiques du projet, pas des prescriptions MPEG, ITU ou EBU. Les tests synthétiques vérifient leur implémentation ; sensibilité et taux de faux positifs n’ont pas été établis sur un corpus musical annoté.

## Interprétation et limites

L’intensity stereo réduit l’information directionnelle tout en gardant les enveloppes d’énergie. Le codage M/S est une représentation somme/différence distincte, qui ne supprime pas intrinsèquement le Side. Voir [ITU HSTP-MCTA, §7.4 et 7.5.2](https://www.itu.int/dms_pub/itu-t/opb/tut/T-TUT-IPTV-2009-MCTA-PDF-E.pdf).

Des gains inégaux ou une opposition de polarité peuvent conserver du Side même si les canaux aigus partagent un signal. Un résultat négatif n’exclut donc pas l’intensity stereo. Les mixages centrés, aigus volontairement étroits, ambiances mono et traitements ultérieurs peuvent produire la même forme. La référence évite d’étiqueter une piste uniformément étroite, sans éliminer tous les cas artistiques.

La mesure nécessite exactement deux canaux, 24–768 kHz et trois demi-secondes admissibles. Mono, multicanal, silence, durée insuffisante, données invalides et anciens rapports donnent un tiret.

## Vérification et exemples

```sh
cargo test -p flaccompagnon-core analysis::intensity_stereo
node --test tests/analysis-cells.test.mjs tests/search.test.mjs
```

Les tests construisent indépendamment des tons Mid/Side : référence à 3 kHz de même énergie, puis Side à 9 kHz soit 40 dB sous Mid, soit aussi fort. Ils vérifient les fuites graves/ultrasoniques, les rapports mono, la persistance, les erreurs, sept taux de 24 à 768 kHz et l’export après décodage WAV.

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*3000*t)+0.1*cos(2*PI*3000*t)+0.101*sin(2*PI*9000*t)|0.1*sin(2*PI*3000*t)-0.1*cos(2*PI*3000*t)+0.099*sin(2*PI*9000*t):s=48000:d=4' -c:a pcm_s24le hf-narrow.wav
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*3000*t)+0.1*cos(2*PI*3000*t)+0.1*sin(2*PI*9000*t)+0.1*cos(2*PI*9000*t)|0.1*sin(2*PI*3000*t)-0.1*cos(2*PI*3000*t)+0.1*sin(2*PI*9000*t)-0.1*cos(2*PI*9000*t):s=48000:d=4' -c:a pcm_s24le hf-wide.wav
```

`hf-narrow.wav` doit montrer un rapport bien plus négatif et l’indice de rétrécissement. Ces signaux contrôlés ne remplacent pas une validation sur de la musique réelle.
