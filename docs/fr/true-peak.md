# Mesure du true peak

Le true peak estime le maximum de l’onde reconstruite entre les échantillons PCM. Une crête stockée inférieure à 0 dBFS peut se reconstruire au-dessus de la pleine échelle. Le résultat s’exprime en dBTP, avec 0 dBTP à pleine échelle.

## Calcul

Chaque canal est suréchantillonné quatre fois à l’aide d’un filtre FIR passe-bas sinc de 48 coefficients, fenêtré par Blackman. L’implémentation polyphasée utilise 12 coefficients par phase et évalue quatre sorties par échantillon entrant. Le maximum absolu sur tous les canaux devient la crête :

```text
dBTP = 20 × log10(oversampled peak)
```

Le silence n’a pas de crête logarithmique finie et est représenté en interne par −∞. Cette mesure est indépendante du compteur de clipping, qui utilise les échantillons stockés.

## Repères d’affichage

| Valeur                    | Repère                |
| ------------------------- | --------------------- |
| Jusqu’à −1 dBTP inclus    | Marge en vert         |
| Au-dessus de −1 jusqu’à 0 | Neutre                |
| Au-dessus de 0 jusqu’à +1 | Avertissement         |
| Au-dessus de +1           | Dépassement important |

Ces couleurs facilitent la comparaison ; elles ne supposent pas des contraintes identiques pour toutes les plateformes et tous les codecs.

## Limites

Le calcul suit l’approche par suréchantillonnage 4× de type BS.1770, avec son propre filtre fixe de 48 coefficients. Ce n’est pas une certification formelle de conformité pour tous les taux, codecs ou filtres de conversion numérique/analogique. Les filtres et traitements de fin de flux peuvent donner de légères différences.

Il s’agit d’une estimation du niveau, pas d’un diagnostic de clipping ou d’un score de distorsion audible.

## Vérification

```sh
cargo test -p flaccompagnon-core analysis::truepeak
```

Les tests comprennent un sinus au quart de la fréquence d’échantillonnage dont les points évitent les sommets : la crête stockée est sous 0,70 tandis que la mesure 4× retrouve plus de 0,95. Vous pouvez comparer le nombre de plateaux et le true peak des exemples de [clipping](clipping.md).

Référence : [ITU-R BS.1770-5](https://www.itu.int/dms_pubrec/itu-r/rec/bs/R-REC-BS.1770-5-202311-I!!PDF-E.pdf).
