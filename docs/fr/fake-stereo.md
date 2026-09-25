# Détection de dual mono

L’indication de fake stereo identifie deux canaux contenant essentiellement le même signal. Elle décrit leur relation, sans constituer un verdict d’authenticité : le dual mono peut être volontaire pour un master mono, des archives ou de la parole.

## Calcul

Le moteur accumule les énergies gauche, droite et de leur différence L−R. Pour un flux d’au moins deux canaux, l’indication apparaît si l’une des conditions suivantes est remplie :

| Condition                                       | Signification                                           |
| ----------------------------------------------- | ------------------------------------------------------- |
| Toutes les trames L/R sont identiques bit à bit | Dual mono exact                                         |
| Énergie L−R à plus de 60 dB sous l’énergie L+R  | Différence stéréo négligeable sur l’ensemble du fichier |

Un flux entièrement silencieux n’est pas qualifié de fake stereo. Au-delà de deux canaux, seuls les deux premiers canaux décodés sont comparés ; aucune conclusion n’est portée sur l’ensemble de la configuration surround.

## Interprétation et limites

Le résultat indique une relation essentiellement mono selon ces règles. Il ne prouve ni une fraude, ni un défaut d’une édition mono, ni qu’une petite différence serait perceptible. Les mixages centrés, ambiances faibles et canaux corrélés peuvent faire varier le résultat autour du seuil technique de 60 dB.

Pour une relation inversée, consultez la [polarité](stereo-polarity.md) ; pour une différence de gain, la [balance](stereo-balance.md).

## Vérification et exemples

```sh
cargo test -p flaccompagnon-core analysis::stereo
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*1000*t)|0.1*sin(2*PI*1000*t):s=48000:d=5' -c:a pcm_s24le dual-mono.wav
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*1000*t)|0.1*sin(2*PI*1700*t):s=48000:d=5' -c:a pcm_s24le stereo.wav
```

Le premier exemple contient deux canaux identiques ; le second contient deux fréquences différentes et ne doit pas être considéré comme dual mono.
