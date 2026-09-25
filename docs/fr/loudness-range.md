# Étendue de loudness (LRA)

La LRA mesure la variation du loudness à court terme dans un programme. Elle suit EBU Tech 3342 et s’exprime en LU. Deux pistes de même LUFS intégré peuvent avoir des variations très différentes. [LUFS-S max](short-term-loudness.md) conserve séparément le maximum local sans porte.

## Calcul

La mesure partage la puissance pondérée K du [LUFS intégré](integrated-loudness.md), mais utilise des fenêtres de trois secondes toutes les 100 ms. À la fin, elle ajoute 1,5 seconde de silence réservée à l’analyse afin que la dernière fenêtre soit centrée sur la fin réelle, conformément à la mesure sur fichier.

Elle conserve les fenêtres à partir de −70 LUFS, calcule leur puissance moyenne, puis conserve les niveaux situés à moins de 20 LU sous cette moyenne. La différence entre les 95e et 10e percentiles restants donne la LRA.

| Propriété      | Réglage                                              |
| -------------- | ---------------------------------------------------- |
| Fenêtre        | 3 secondes                                           |
| Intervalle     | 100 ms ou plus fréquent selon le taux pris en charge |
| Porte absolue  | −70 LUFS                                             |
| Porte relative | Moyenne moins 20 LU                                  |
| Étendue        | 95e percentile moins 10e percentile                  |

Seuls les flux mono ou stéréo valides à 8–768 kHz sont mesurés. Moins d’une fenêtre complète de trois secondes, silence, données invalides ou puissance interne non représentable donnent une absence de résultat.

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

Attendez environ 10 LU, avec l’indication d’approximation sous 60 secondes. Comparez avec FFmpeg sans exiger un arrondi strictement identique.

Références : [EBU Tech 3342](https://tech.ebu.ch/docs/tech/tech3342.pdf), [EBU Tech 3341](https://tech.ebu.ch/docs/tech/tech3341.pdf).
