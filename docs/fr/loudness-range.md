# Étendue de loudness (LRA)

La LRA mesure la variation du loudness à court terme dans un programme. Elle suit EBU Tech 3342 et s’exprime en LU. Deux pistes de même LUFS intégré peuvent avoir des variations très différentes. [LUFS-S max](short-term-loudness.md) conserve séparément le maximum local sans porte.

## Calcul

La mesure partage la puissance pondérée K du [LUFS intégré](integrated-loudness.md), mais utilise des fenêtres de trois secondes toutes les 100 ms. Sur fichier, elle suit la convention de référence de Tech 3342 en ajoutant au moins 1,5 seconde de silence réservée à l’analyse, arrondie au nombre entier supérieur de trames. Le centre de la dernière fenêtre se situe à moins d’une mise à jour de la fin réelle ; une fin de fichier arbitraire ne coïncide pas forcément avec la grille. Ce silence traverse les filtres et conserve donc leur réponse résiduelle.

Elle conserve les fenêtres à partir de −70 LUFS, calcule leur puissance moyenne, puis conserve les niveaux supérieurs ou égaux à cette moyenne moins 20 LU. La différence entre les 95e et 10e percentiles restants donne la LRA. Pour `n` valeurs conservées et triées par ordre croissant, les rangs à partir de zéro sont `round((n − 1) × 0.10)` et `round((n − 1) × 0.95)`, sans interpolation. La LRA inclut les valeurs exactement égales aux seuils, contrairement au LUFS intégré.

| Propriété      | Réglage                                              |
| -------------- | ---------------------------------------------------- |
| Fenêtre        | 3 secondes                                           |
| Intervalle     | 100 ms ou plus fréquent selon le taux pris en charge |
| Porte absolue  | −70 LUFS                                             |
| Porte relative | Moyenne moins 20 LU                                  |
| Étendue        | 95e percentile moins 10e percentile                  |

Seuls les flux mono ou stéréo valides à 8–768 kHz sont mesurés. Moins d’une fenêtre complète de trois secondes, silence, données invalides ou puissance interne non représentable donnent une absence de résultat.

Les historiques écartent les valeurs qui ne peuvent jamais franchir la porte absolue. La LRA réutilise son vecteur et sélectionne deux rangs sans tri complet ni deuxième allocation de taille proportionnelle au programme. Les tampons partagés de 400 ms et 3 s occupent environ 11,1 Mio à 768 kHz ; les historiques retenus croissent d’environ 160 octets par seconde de programme, hors capacité supplémentaire des vecteurs. Les maxima M/S ajoutent un état de taille fixe. Des sommes glissantes compensées préservent les fenêtres calmes après un passage flottant très fort, sans fournir une précision arbitraire.

## Interprétation et limites

Une LRA élevée indique davantage de variation parmi les niveaux locaux retenus. Ce n’est pas un rapport crête/RMS et donc pas le DR. Elle ne détermine pas si une compression est artistiquement adaptée, ni le volume ressenti dans tous les contextes d’écoute.

L’EBU considère les programmes de moins de 60 secondes comme une approximation pour cette mesure ; l’application les signale. Un sinus constant de cinq secondes ne constitue pas un bon test exigeant exactement zéro LRA : les fenêtres finales et les portes pèsent beaucoup sur un programme très court.

## Vérification et exemple

```sh
cargo test -p flaccompagnon-core analysis::loudness
cargo test -p flaccompagnon-core --test loudness_reference -- --ignored --nocapture
ffmpeg -n -f lavfi -i 'aevalsrc=if(lt(t\,20)\,0.1\,0.0316227766)*sin(2*PI*1000*t)|if(lt(t\,20)\,0.1\,0.0316227766)*sin(2*PI*1000*t):s=48000:d=40' -c:a pcm_s24le range.wav
ffmpeg -i range.wav -af ebur128 -f null -
```

Les tests couvrent les quatre distributions de niveaux publiées dans EBU Tech 3342, la porte basée sur la moyenne en puissance, l’égalité au seuil absolu, l’exclusion des percentiles, les observations répétées, le silence et l’arrondi final aux taux impairs. Le test facultatif utilise les métadonnées FFmpeg à trois décimales, à plusieurs taux. Les conventions de percentiles et de fin peuvent différer entre les deux mesures ; la comparaison LRA utilise la tolérance d’acceptation de ±1 LU de Tech 3342.

Attendez environ 10 LU, avec l’indication d’approximation sous 60 secondes. Comparez avec FFmpeg sans exiger un arrondi strictement identique.

Références : [EBU Tech 3342](https://tech.ebu.ch/docs/tech/tech3342.pdf), [EBU Tech 3341](https://tech.ebu.ch/docs/tech/tech3341.pdf), [conseils sur les programmes courts dans EBU Tech 3343](https://tech.ebu.ch/docs/tech/tech3343.pdf).
