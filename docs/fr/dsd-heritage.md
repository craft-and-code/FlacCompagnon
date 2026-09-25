# Héritage DSD dans le PCM haute résolution

Cette analyse reconnaît une forme spectrale associée à un master DSD converti en PCM haute résolution. Elle fournit un indice pour le badge descriptif `Hi-Res (DSD source)`, sans créer d’alerte d’authenticité ni prouver une origine DSD.

## Calcul

Le test concerne le PCM à partir de 96 kHz. Dans le spectre moyen, il mesure une vallée entre 22 et 30 kHz puis une remontée entre 36 kHz et le minimum de 75 kHz ou 92 % de Nyquist.

| Condition                    | Seuil                                                |
| ---------------------------- | ---------------------------------------------------- |
| Remontée ultrasonique        | Au moins 15 dB au-dessus de la vallée                |
| Niveau absolu de la remontée | Au-dessus de −75 dB par rapport à la crête spectrale |

Les deux bandes doivent contenir suffisamment de cases FFT finies. La moyenne sur une large bande évite qu’un petit nombre de cases isolées suffise à suggérer une origine.

## Interprétation et limites

Le procédé sigma-delta du DSD peut créer une remontée de bruit ultrasonique qui survit à une conversion PCM à taux suffisamment élevé. Le test cherche cette forme. Un autre procédé peut produire la même vallée et la même remontée ; à l’inverse, un filtrage passe-bas, une réduction de bruit ou un traitement ultérieur peuvent l’effacer.

Un résultat absent ne prouve donc pas une source native PCM. Le contrôle des fichiers DSD est distinct : voir [source PCM dans le DSD](dsd-pcm-source.md).

## Vérification

```sh
cargo test -p flaccompagnon-core dsd::spectral
cargo test -p flaccompagnon-core pipeline
```

Importez un transfert PCM à haute fréquence d’échantillonnage dont l’origine DSD est documentée, puis examinez le spectre et le résultat ensemble. Le badge ne doit pas être utilisé comme certificat de provenance.
