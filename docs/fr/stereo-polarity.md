# Polarité stéréo

La mesure compare les deux canaux décodés sur l’ensemble du fichier. Elle signale une inversion probable de polarité gauche/droite lorsqu’ils sont fortement opposés. Elle ne détermine pas la polarité absolue et ne décide pas si un effet artistique est un défaut.

## Calcul

Pour exactement deux canaux, le moteur accumule les énergies gauche `E_L`, droite `E_R` et croisée `C = Σ(L×R)` :

```text
correlation = C / sqrt(E_L × E_R)
```

Le résultat est borné entre −1 et +1. À partir de −0,95 vers −1, l’indication de polarité apparaît. La relation `R = −kL` est acceptée même si les gains diffèrent et empêchent une annulation complète en mono.

Une énergie invalide, un canal silencieux ou une configuration autre que stéréo rendent la corrélation indisponible.

## Interprétation et limites

Près de +1, les canaux évoluent ensemble ; près de 0, leur corrélation linéaire globale est faible ; près de −1, ils s’opposent. Le seuil est volontairement conservateur : une valeur supérieure à −0,95 reste informative pour le mono sans être étiquetée comme inversion.

Une courte section inversée peut disparaître dans la moyenne. Ambiances, délais et traitements stéréo intentionnels empêchent d’en faire une note de qualité. La [phase locale et par bande](local-phase.md) révèle des oppositions masquées par cette moyenne. Écoutez en mono et examinez la source avant de modifier le son.

## Vérification et exemple

```sh
cargo test -p flaccompagnon-core analysis::stereo
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*1000*t)|-0.1*sin(2*PI*1000*t):s=48000:d=5' -c:a pcm_s24le polarity.wav
```

La corrélation doit être proche de −1 et l’indication apparaître. Dans Audacity, inversez un seul canal puis exportez une copie : l’indication doit disparaître.
