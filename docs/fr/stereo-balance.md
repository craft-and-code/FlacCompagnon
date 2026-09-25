# Balance stéréo

La balance indique l’écart de niveau RMS entre les deux canaux sur toute la piste. Panoramique, placement des microphones, mastering et décalage continu peuvent créer un écart sans défaut d’enregistrement.

## Calcul

Le moteur accumule l’énergie non pondérée de chaque canal. Comme les deux comptent le même nombre de trames :

```text
right minus left = 10 × log10(E_R / E_L) dB
```

Un résultat positif signifie que la droite est plus forte ; un résultat négatif, la gauche. L’affichage utilise `R +x.x dB` ou `L +x.x dB`. Un canal exactement nul est indiqué `L silent` ou `R silent`, sans exporter d’infini. Deux canaux silencieux, des valeurs invalides ou un format non stéréo n’ont pas de mesure.

Le CSV conserve la différence signée `balance_right_minus_left_db` et l’état du canal silencieux dans `balance_silent_channel`.

## Limites

Cette mesure RMS globale, sur toute la bande et sans pondération, ne modélise pas le loudness humain. Elle ne vérifie pas l’affectation des canaux et ne distingue pas un mixage asymétrique d’un problème de câblage. Un événement fort peut dominer l’énergie ; un déséquilibre limité à certaines fréquences peut être masqué.

## Vérification et exemples

```sh
cargo test -p flaccompagnon-core analysis::stereo
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*1000*t)|0.05*sin(2*PI*1000*t):s=48000:d=5' -c:a pcm_s24le balance-left.wav
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*1000*t)|0:s=48000:d=5' -c:a pcm_s24le right-silent.wav
```

Attendez `L +6.0 dB` pour le premier, soit exactement 6,0206 dB par le rapport d’amplitude, et `R silent` sans corrélation pour le second. Vérification indépendante : `ffmpeg -i balance-left.wav -af astats -f null -`, en comparant les RMS par canal.
