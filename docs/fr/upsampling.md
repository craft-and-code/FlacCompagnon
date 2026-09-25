# Détection du suréchantillonnage

`Upsampled` indique qu’un conteneur PCM à haute fréquence possède une zone haute durablement vide, compatible avec une source à fréquence plus faible. Un master natif limité en bande ou filtré volontairement peut produire les mêmes échantillons : c’est une heuristique, pas une preuve d’historique.

## Signal testé

Le détecteur s’applique uniquement au-dessus de 48 kHz. Il examine des fenêtres MDCT de taille AAC, distinctes du cutoff FFT affiché. Cette vue permet de vérifier la persistance d’une zone vide dans des fenêtres locales, au lieu de considérer une case spectrale isolée et faible comme du contenu significatif.

| Condition              | Seuil de projet                                     |
| ---------------------- | --------------------------------------------------- |
| Conteneur à taux élevé | Plus de 48 kHz                                      |
| Zone vide persistante  | Au moins 70 % des fenêtres MDCT analysées           |
| Coupure MDCT moyenne   | Sous 90 % de Nyquist et au plus 30 kHz              |
| Niveau moyen au-dessus | Au plus −75 dB par rapport à la crête de la fenêtre |

L’analyse prélève un pas MDCT sur quatre et se limite à 240 fenêtres. Le temps de calcul reste borné ; le résultat concerne cet échantillonnage du programme, pas toutes les fenêtres possibles.

## Résultat et interprétation

Le détail donne la frontière estimée, le taux du conteneur et rappelle que la bande limitée ne prouve pas le rééchantillonnage. Clean signifie que la signature n’a pas été établie. Bruit ultrasonique, traitement ultérieur ou autre forme de rééchantillonnage peuvent empêcher une détection.

Le cutoff affiché n’est volontairement pas l’entrée du verdict. Le FFT moyen peut aller jusqu’à Nyquist à cause de bruit isolé, alors que la plupart des fenêtres MDCT sont vides en haut. Il peut aussi être bas sur un enregistrement natif naturellement sombre. La description spectrale reste donc distincte du verdict.

## Limites

Deux fichiers de mêmes échantillons mais d’historiques différents sont indiscernables par ce test. Acoustique, archives, bandes analogiques et passe-bas peuvent réduire les ultrasons ; du bruit ajouté peut masquer une source rééchantillonnée. Les seuils sont des choix calibrés du projet, pas une norme de classification ni une probabilité de suréchantillonnage.

Le DSD utilise une autre mesure adaptée à son bruit ultrasonique : [source PCM dans le DSD](dsd-pcm-source.md).

## Vérification et exemple

```sh
cargo test -p flaccompagnon-core analysis::detections
cargo test -p flaccompagnon-core analysis::analyzer
ffmpeg -n -f lavfi -i 'sine=frequency=1000:sample_rate=44100:duration=30' -af 'lowpass=f=20000,aresample=96000' -c:a pcm_s24le upsampled-fixture.wav
```

Cet exemple crée une source limitée puis rééchantillonnée à 96 kHz. Il vérifie le chemin de traitement et l’affichage, sans établir une précision universelle sur la musique.
