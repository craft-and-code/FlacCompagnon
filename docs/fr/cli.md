# Utiliser la ligne de commande

La **CLI** permet d’utiliser FlacCompagnon sans fenêtre graphique : vous saisissez une commande dans Terminal ou PowerShell, vous indiquez un fichier ou un dossier, et les résultats s’affichent au même endroit. Elle utilise le même moteur Rust et le même format de rapport JSON que l’application. L’analyse lit vos fichiers audio sans les modifier.

Avec uniquement `-a flac-md5`, sans `--json` ni `--json-layout`, la CLI se limite au décodage et au contrôle de l’intégrité des FLAC. Elle évite les analyses spectrales, la détection de transcodage et les empreintes du fichier entier. Un échec de comparaison MD5 ou de décodage donne un code de sortie non nul. Les autres sélections explicites exécutent elles aussi uniquement les mesures demandées. Les exports JSON conservent l’analyse complète. Ce contrôle utilise `flac -t` si l’exécutable optionnel `flac` est disponible dans le `PATH`, sinon le vérificateur Rust interne.

## Installer et vérifier

Dans les [Releases GitHub](https://github.com/craft-and-code/FlacCompagnon/releases), choisissez une archive autonome nommée `FlacCompagnon-CLI_<version>_<plateforme>.tar.gz` ou `.zip`. Les fichiers de l’application graphique commencent par `FlacCompagnon-App`, pour distinguer les deux téléchargements. Les plateformes prévues sont macOS Apple Silicon, Linux x86_64 et Windows x64.

Décompressez l’archive. Depuis son dossier, lancez :

```sh
# macOS ou Linux
./flaccompagnon --help
./flaccompagnon --version
```

```powershell
# Windows PowerShell
.\flaccompagnon.exe --help
.\flaccompagnon.exe --version
```

Les exemples suivants supposent que le dossier de l’exécutable figure dans votre `PATH`, la liste des dossiers où le terminal cherche ses commandes. Sinon, remplacez `flaccompagnon` par `./flaccompagnon`, `.\flaccompagnon.exe` ou son chemin complet. Sur macOS, suivez les instructions de premier lancement de la release si le système bloque cette version non signée.

## Analyser un premier fichier

```sh
flaccompagnon "Music/Album/01 - Track.flac"
```

La progression indique chaque fichier avant son analyse. Les mesures restent masquées par défaut ; `--show-results` les affiche. Un indicateur animé accompagne la préparation et le décodage dans un terminal interactif. Les guillemets conservent ensemble un chemin contenant des espaces. Une valeur Rust indisponible peut apparaître sous la forme `None` : ce n’est pas un zéro mesuré. La progression est envoyée sur stderr et les mesures demandées sur stdout.

Pour analyser un album, y compris ses sous-dossiers :

```sh
flaccompagnon "Music/Album"
```

Vous pouvez mélanger plusieurs fichiers et dossiers. Pour rester au premier niveau des dossiers indiqués :

```sh
flaccompagnon "Music/Album A" "Music/Album B" --no-recursive
```

## Choisir les analyses à exécuter

```sh
flaccompagnon "Music/Album" --analysis loudness
flaccompagnon "Music/Album" --analysis phase --analysis hf-stereo
flaccompagnon "Music/Album" -a clipping,clicks,dropouts
```

**Sans export JSON, la sélection contrôle les calculs et l’affichage du terminal.** Seules les mesures demandées et leurs calculs préparatoires nécessaires sont exécutés. Les virgules et les options `--analysis` répétées permettent de combiner autant de mesures que nécessaire. Par exemple, `-a flac-md5,loudness` décode une seule fois pour vérifier l’intégrité audio et mesurer la loudness, sans analyse spectrale ni détection de transcodage. Sans sélection, toutes les analyses sont exécutées. `--json` et `--json-layout` exigent toutes les analyses et ne peuvent pas être combinés avec `--analysis` (`-a`). La CLI refuse cette combinaison avant de parcourir les fichiers, avec le code de sortie 2 ; elle n’élargit pas silencieusement votre sélection.

| Nom            | Résultats dans le terminal                                          |
| -------------- | ------------------------------------------------------------------- |
| `authenticity` | Résumé d’authenticité et détails des détections                     |
| `bit-depth`    | Profondeur déclarée et profondeur effective                         |
| `spectrum`     | Coupure spectrale                                                   |
| `stereo`       | Dual mono et balance des canaux                                     |
| `phase`        | Corrélation globale, polarité et phase locale/par bande             |
| `hf-stereo`    | Mesure de la stéréo dans les aigus                                  |
| `clipping`     | Événements de clipping, crête des échantillons et true peak         |
| `loudness`     | LUFS intégré, maxima M/S et LRA                                     |
| `dynamics`     | Estimation DR                                                       |
| `clicks`       | Nombre d’impulsions suspectes (colonne Impulses dans l’application) |
| `dropouts`     | Nombre de coupures suspectes                                        |
| `dc-offset`    | Décalage continu des canaux                                         |
| `flac-md5`     | État de la signature audio FLAC                                     |
| `fingerprints` | MD5 et CRC32 du fichier complet                                     |

## Enregistrer et rouvrir un rapport JSON

```sh
flaccompagnon "Music/Album" --json "Music/Album/FlacCompagnon.json"
```

`--json` exige un **chemin de fichier**, pas seulement un dossier ou une option sans valeur. Le dossier parent doit déjà exister. Les résultats valides de fichiers inchangés sont réutilisés. `--force` refait les analyses et remplace le rapport sélectionné. Un JSON de destination invalide nécessite cette option. Un seul rapport regroupe tous les fichiers audio sélectionnés : `--json-layout album` ou `--json-layout artist` permet de créer un rapport par album.

Le schéma est le format complet et versionné `flaccompagnon-report` de l’application. Déposez le JSON sur sa liste de résultats pour rouvrir l’analyse sans redécoder le son. Le rapport est un instantané : il ne s’actualise pas après une modification du son et ses chemins peuvent devenir obsolètes après un déplacement. Consultez les [empreintes de fichiers](fingerprints.md) pour les possibilités et limites d’identification.

Un export JSON exige toujours toutes les analyses. Pour afficher les mesures complètes tout en les enregistrant, utilisez `--show-results` :

```sh
flaccompagnon "Music/Album" --show-results -j "Music/Album/FlacCompagnon.json"
```

## Un rapport par album avec Aède

[Aède](https://craft-and-code.github.io/aede/) est un catalogue de bibliothèque musicale écrit en Rust. Il organise les pistes et leurs métadonnées et intègre directement le moteur FlacCompagnon. Il n’a pas besoin de lancer cet exécutable.

Avec une version d’Aède qui inclut cette intégration, commencez par cataloguer les fichiers, puis analysez-les :

```sh
aede scan "Music"
aede analyze "Music" --json
```

Aède analyse les pistes cataloguées et écrit un `<album>.json` dans chaque dossier d’album, selon les regroupements de son catalogue, y compris le dossier commun des albums multidisques. Un rapport existant peut être importé avec `aede import "Music/Album/FlacCompagnon.json"`. Le README d’Aède précise les options de sa version.

## DSD et FFmpeg

L’analyse PCM courante ne nécessite pas FFmpeg. L’analyse du contenu DSD l’utilise ; sans décodeur disponible, les informations du conteneur restent lisibles, mais les mesures du contenu sont indisponibles.

```sh
flaccompagnon "Music/DSD Album" --ffmpeg "/opt/homebrew/bin/ffmpeg"
```

La recherche utilise d’abord `--ffmpeg`, puis la variable d’environnement `FLACCOMPAGNON_FFMPEG`, puis le `PATH`. La CLI ne génère pas de spectrogrammes et ne modifie pas les tags.

## Référence des options

```text
flaccompagnon [OPTIONS] <FILE|FOLDER>...
```

| Option                        | Effet                                                                          |
| ----------------------------- | ------------------------------------------------------------------------------ |
| `-a, --analysis NAME`         | Exécute les analyses choisies ; répétable ou noms séparés par des virgules     |
| `--show-results`              | Affiche les mesures dans le terminal                                           |
| `--force`                     | Refait les analyses et remplace les rapports sélectionnés                      |
| `--json-layout album\|artist` | Un rapport par album dans son dossier ou son parent                            |
| `-j, --json PATH`             | Exécute toutes les analyses et écrit le rapport JSON complet                   |
| `--recursive`                 | Inclut les sous-dossiers, par défaut                                           |
| `--no-recursive`              | Reste au niveau du dossier indiqué                                             |
| `--no-flac-md5`               | Lit les signatures FLAC sans les comparer à l’audio décodé                     |
| `--ffmpeg PATH`               | Choisit l’exécutable FFmpeg pour le DSD                                        |
| `-h, --help`                  | Affiche l’aide                                                                 |
| `-v, -V, --version`           | Affiche la version ; s’utilise seul                                            |
| `--`                          | Traite les arguments suivants comme des chemins, même s’ils commencent par `-` |

## Codes de sortie et automatisation

| Code | Signification                                                                |
| ---- | ---------------------------------------------------------------------------- |
| `0`  | Traitement terminé sans erreur de fichier                                    |
| `1`  | Échec du traitement, de l’écriture du rapport ou d’au moins un fichier audio |
| `2`  | Arguments de commande invalides                                              |

Une détection d’authenticité ou un événement de clipping ne provoque **pas** le code 1. Un script doit consulter les résultats JSON pour réagir à ces observations. Un rapport peut contenir des lignes en erreur et être écrit avant la sortie avec le code 1. Sa seule existence ne garantit donc pas la réussite.

## Compiler et utiliser la bibliothèque Rust

Depuis la racine du dépôt, avec Rust installé :

```sh
cargo build --release -p flaccompagnon-cli
./target/release/flaccompagnon --help
```

Sur Windows, l’exécutable est `target\release\flaccompagnon.exe`. Cette compilation ne nécessite pas l’environnement graphique Tauri. Un autre logiciel Rust peut dépendre directement de `flaccompagnon-core` ; ses fonctions et types de rapports sont décrits dans [Rustdoc](https://craft-and-code.github.io/FlacCompagnon/doc/). Une dépendance Git nécessite un commit ou un tag envoyé sur le dépôt, sans attendre la fin de la compilation des binaires de release.

## Enregistrer les rapports

```sh
flaccompagnon "Musique/Groupe" --json-layout album
flaccompagnon "Musique/Groupe" --json-layout artist
flaccompagnon "Musique/Album" --json "Musique/Album/rapport.json"
```

`album` enregistre `<album>.json` dans chaque dossier d’album. `artist` enregistre ces fichiers séparés dans leur dossier parent, celui du groupe. Le regroupement repose sur les dossiers audio ; les sous-dossiers numérotés `CD`, `Disc` et `Disk` sont regroupés dans leur album parent. Il ne repose pas sur les tags. `--json-layout` active l’enregistrement et ne se combine pas avec `--analysis` ni avec `--json PATH`, qui produit un rapport unique. Le dossier de destination doit exister. Les JSON complets peuvent être réimportés dans l’application ou Aède.

## Réutiliser ou refaire une analyse

Les résultats réussis présents dans les rapports d’album ou de leur dossier parent sont réutilisés lorsque le chemin, la taille et la date de modification correspondent. Cela fonctionne aussi sans option d’enregistrement. Les fichiers nouveaux, modifiés ou précédemment en erreur sont analysés.

```sh
flaccompagnon "Musique/Groupe" --json-layout album --force
```

`--force` refait les analyses et remplace les rapports sélectionnés pour l’enregistrement. Une destination contenant un JSON invalide nécessite cette option. Ces contrôles reposent sur les métadonnées : utilisez également `--force` après une modification qui conserve taille et date, ou pour recalculer avec une nouvelle version du moteur.

Aède accepte les mêmes options `--force`, `--show-results` et `--json-layout album|artist` avec `aede analyze`. `--json` conserve l’enregistrement dans chaque album.
