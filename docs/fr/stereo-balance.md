# Balance stéréo

La balance indique l’écart de niveau RMS entre les deux canaux sur toute la piste. Panoramique, placement des microphones, mastering et décalage continu peuvent créer un écart sans défaut d’enregistrement.

## Calcul

Le moteur accumule l’énergie non pondérée de chaque canal. Comme les deux comptent le même nombre de trames :

```text
right minus left = 10 × log10(E_R / E_L) dB
```

Cela suit la [définition ordinaire du RMS](https://www.mathworks.com/help/dsp/ref/rms.html). Le code soustrait les deux logarithmes sans diviser d’abord les énergies, pour éviter un débordement du rapport avec des niveaux très inégaux. Un gain commun ou l’inversion de polarité de l’un des canaux ne change pas la balance ; permuter les canaux inverse son signe. Sans durée minimale ni porte absolue, un signal flottant court et très faible mais non nul reste mesurable.

Un résultat positif signifie que la droite est plus forte ; un résultat négatif, la gauche. L’affichage utilise `R +x.x dB` ou `L +x.x dB`. Un canal exactement nul est indiqué `L silent` ou `R silent`, sans exporter d’infini. Deux canaux silencieux, des valeurs invalides ou un format non stéréo n’ont pas de mesure.

Le CSV conserve la différence signée `balance_right_minus_left_db` et l’état du canal silencieux dans `balance_silent_channel`.

## Limites

Cette mesure RMS globale, sur toute la bande et sans pondération, ne modélise pas le loudness humain. Elle ne vérifie pas l’affectation des canaux et ne distingue pas un mixage asymétrique d’un problème de câblage. Un événement fort peut dominer l’énergie ; un déséquilibre limité à certaines fréquences peut être masqué.

Le DC est volontairement inclus : `moyenne des carrés = variance + moyenne²`. Deux canaux d’énergie AC égale mais de biais constants différents peuvent donc donner un écart. C’est une balance brute du signal, pas une comparaison de niveau AC seul. Consultez séparément le [DC offset](dc-offset.md) ; le retirer ici changerait silencieusement la quantité mesurée. Une différence de loudness demanderait également sa propre définition de pondération, durée et portes, distincte du [loudness de programme ITU-R BS.1770-5](https://www.itu.int/dms_pubrec/itu-r/rec/bs/R-REC-BS.1770-5-202311-I!!PDF-E.pdf).

Un RMS égal ne prouve ni des canaux identiques, ni un loudness égal, ni une annulation mono. Il faut examiner la relation des formes d’onde, leur contenu fréquentiel et les [mesures de phase](local-phase.md). Aucun seuil bon/mauvais n’est appliqué. L’accumulation garde une mémoire constante et deux sommes d’énergie dans le décodage existant ; les deux logarithmes ne sont calculés qu’à la fin.

## Vérification et exemples

```sh
cargo test -p flaccompagnon-core analysis::stereo
cargo test -p flaccompagnon-core --test stereo_balance --test stereo_validity
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*1000*t)|0.05*sin(2*PI*1000*t):s=48000:d=5' -c:a pcm_s24le balance-left.wav
ffmpeg -n -f lavfi -i 'aevalsrc=0.1*sin(2*PI*1000*t)|0:s=48000:d=5' -c:a pcm_s24le right-silent.wav
```

Les tests analytiques couvrent le passage amplitude/énergie, le gain commun, les rapports extrêmes, l’énergie indépendante de la polarité, le DC brut et le silence exact. Les WAV flottants couvrent des extraits de seize trames très faibles ou au-dessus de la pleine échelle, des biais connus et les canaux silencieux. Des injections de trames invalides vérifient que le préfixe valide n’est pas publié après une erreur stéréo.

Attendez `L +6.0 dB` pour le premier, soit exactement 6,0206 dB par le rapport d’amplitude, et `R silent` sans corrélation pour le second. Vérification indépendante : `ffmpeg -i balance-left.wav -af astats -f null -`, en comparant les RMS par canal.
