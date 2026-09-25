# Mesure de la coupure spectrale

Le cutoff décrit la plus haute fréquence portant encore une énergie appréciable dans le spectre moyen. Une valeur basse peut venir de la musique, d’archives ou d’un choix de filtrage ; une valeur haute peut venir du bruit. Ce n’est pas, isolément, un verdict d’authenticité.

## Calcul

Les trames décodées sont mélangées en mono. Une FFT de 8192 points, avec fenêtre de Hann, accumule la puissance par case puis convertit le spectre moyen en décibels relatifs à sa case la plus forte. À 44,1 kHz, l’espacement nominal des cases est d’environ 5,4 Hz.

Une moyenne mobile de cinq cases réduit l’effet des valeurs isolées. La coupure est la case lissée la plus haute dépassant −90 dB par rapport à la crête spectrale. Environ 3 kHz de part et d’autre servent aux mesures complémentaires :

| Valeur | Signification                                                               |
| ------ | --------------------------------------------------------------------------- |
| Cutoff | Fréquence la plus haute dépassant le plancher                               |
| Cliff  | Niveau moyen sous la coupure moins celui au-dessus ; positif pour une chute |
| Above  | Niveau moyen au-dessus, relatif à la crête spectrale                        |

Les entrées partielles sont complétées par des zéros pour produire un spectre des fichiers courts. La précision fréquentielle reste limitée par la durée réellement observée.

## Interprétation et limites

La mesure décrit une réponse moyenne et oriente l’inspection du spectrogramme. Elle reste distincte du [transcodage](transcoding.md), fondé sur les grilles de codecs, et de l’[upsampling](upsampling.md), fondé sur des zones vides MDCT locales.

Une coupure près de Nyquist indique seulement des cases au-dessus du plancher. Une coupure basse indique leur absence dans le spectre moyen. Ni l’une ni l’autre ne prouve la façon dont le son a été enregistré ou encodé.

Le seuil relatif permet à un faible bruit aigu de prolonger la coupure. Un enregistrement natif sombre peut au contraire la réduire. Le mélange mono peut annuler une partie du contenu stéréo ; le spectre moyen ne localise pas les changements de fréquence dans le temps.

## Vérification et exemple visuel

```sh
cargo test -p flaccompagnon-core analysis::spectrum
ffmpeg -n -f lavfi -i 'anoisesrc=color=white:amplitude=0.2:sample_rate=48000:duration=10:seed=42' -c:a pcm_s24le broadband-fixture.wav
ffmpeg -n -i broadband-fixture.wav -af 'lowpass=f=8000' -c:a pcm_s24le lowpass-fixture.wav
```

Comparez les spectres et spectrogrammes : le second doit atténuer les aigus. Ce passe-bas a une pente progressive, pas une coupure idéale ; il ne faut pas exiger un cutoff exactement à 8 kHz. Un sinus seul à 1 kHz ne constitue pas un témoin large bande adapté à cette comparaison.
