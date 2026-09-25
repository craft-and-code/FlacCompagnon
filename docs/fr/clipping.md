# Clipping des échantillons

Cette analyse recherche des suites d’échantillons PCM stockés à pleine échelle ou très proches. Le [true peak](true-peak.md) estime séparément les dépassements entre les échantillons lors de la reconstruction.

## Calcul

Tous les échantillons de tous les canaux sont comparés au seuil absolu normalisé `0.9997`. Tous ceux qui l’atteignent sont comptés. Une suite devient un événement de clipping à partir de trois échantillons consécutifs ; une suite plus longue reste un seul événement. Un sommet isolé ne suffit donc pas à créer un plateau détecté.

Le résultat contient une indication booléenne, le nombre d’événements, le nombre d’échantillons concernés et la crête ordinaire. Le comptage des suites suit l’ordre des échantillons entrelacés décodés : des plateaux simultanés dans différents canaux sont représentés par cette séquence, sans supposer un événement acoustique commun.

## Interprétation et limites

Un événement indique un plateau soutenu proche de la pleine échelle. Il ne dit pas s’il est audible, involontaire, acquis à l’enregistrement ou créé comme effet. Le seuil est une tolérance technique, pas une règle universelle de mastering. Une suite de moins de trois échantillons augmente le comptage des échantillons proches du plafond, sans créer d’événement.

Un fichier sans clipping de ses échantillons peut avoir un true peak positif. Inversement, le true peak d’un fichier écrêté dépend du filtre de reconstruction. Consultez les deux mesures pour évaluer la marge disponible.

## Vérification et exemples

```sh
cargo test -p flaccompagnon-core analysis::clipping
ffmpeg -n -f lavfi -i 'aevalsrc=0.5*sin(2*PI*1000*t):s=48000:d=5' -c:a pcm_s24le clean-level.wav
ffmpeg -n -f lavfi -i 'aevalsrc=clip(1.5*sin(2*PI*1000*t)\,-1\,1):s=48000:d=5' -c:a pcm_s24le clipped-level.wav
```

Le second fichier doit montrer des événements et de nombreux échantillons proches du plafond. Dans Audacity, zoomez sur ses sommets plats.
