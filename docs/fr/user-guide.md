# Guide de l’utilisateur

FlacCompagnon rassemble l’analyse d’une bibliothèque, la lecture, les tags et la conversion au même endroit. Ce guide suit un usage simple : déposer des morceaux, parcourir les résultats, puis intervenir uniquement là où vous le souhaitez.

## Avant de commencer

Vous pouvez déposer un fichier audio ou un dossier entier. Les sous-dossiers sont parcourus et les fichiers déjà présents dans la liste ne sont pas ajoutés une seconde fois.

L’analyse ne modifie jamais vos fichiers. Les écritures sur le disque demandent une action claire de votre part : enregistrer des tags ou une image, renommer ou renuméroter une piste, convertir, générer des spectrogrammes ou enregistrer un rapport.

## Analyser des fichiers par glisser-déposer

1. Ouvrez FlacCompagnon.
2. Glissez un dossier ou des fichiers audio sur la grande zone centrale.
3. Attendez la fin de l’analyse. La liste indique les fichiers, leurs caractéristiques et les résultats utiles.

Le bouton **Add titles…** permet aussi de choisir un dossier dans le sélecteur de fichiers. C’est pratique si vous préférez ne pas utiliser le glisser-déposer.

Chaque ligne représente une piste. Les colonnes visibles décrivent par exemple le format, la fréquence d’échantillonnage, le nombre de bits, une éventuelle détection et les mesures de niveau. Un résultat attire votre attention sur une caractéristique mesurée ; il ne remplace pas l’écoute.

> **Repère à l’écran :** utilisez la petite loupe d’une ligne pour retrouver le fichier dans le Finder ou l’Explorateur. Le bouton de lecture voisin lance directement cette piste.

Pour renommer une piste, sélectionnez sa ligne, puis cliquez à nouveau sur son nom. **Enter** applique le nouveau nom en conservant l’extension ; **Escape** annule. Le renommage ne remplace jamais un fichier de destination existant.

Pour comprendre une colonne, ouvrez l’article correspondant dans la section [Analyses](index.md). Chaque article explique ce que la mesure regarde et ses limites.

## Modifier les tags et les images

Sélectionnez une ou plusieurs pistes dans la liste. Le volet **Tags** apparaît à gauche.

Vous y retrouvez l’image de l’album et les champs courants : titre, artiste, album, artiste de l’album, année, genre, numéro de piste et commentaire. Modifiez un ou plusieurs champs, puis utilisez **Save** au bas du volet pour écrire les changements dans les fichiers sélectionnés.

L’image suit le rôle choisi dans la liste sous la jaquette, par exemple _Front cover_ ou _Back cover_. Changez ce rôle pour afficher l’image correspondante. Vous pouvez :

- déposer une image sur la jaquette pour la préparer pour les pistes sélectionnées ;
- utiliser le bouton d’export sous l’image pour l’extraire près des fichiers audio ;
- utiliser la corbeille pour préparer sa suppression.

Les images locales et intégrées sont limitées à 12 Mio et 64 millions de pixels. Une image trop grande doit être redimensionnée avant d’être ajoutée.

L’en-tête doit fournir des dimensions valides. Le TIFF classique est pris en charge ; BigTIFF, les TIFF avec SubIFDs et les BMP contenant un JPEG/PNG sont refusés. Ce contrôle lit la géométrie de l’image sans décoder tous ses pixels.

Les changements restent en attente tant que vous n’avez pas choisi **Save**. Le bouton **Reset** annule seulement les changements en attente.

## Convertir une sélection

Le bouton avec les deux flèches, dans la barre du haut, ouvre le volet **Convert** à droite.

Déposez directement des fichiers ou un dossier dans ce volet pour créer une liste de conversion indépendante. Si les pistes ont déjà été analysées et sélectionnées, utilisez **Add selected** pour les ajouter à cette liste.

Choisissez le format voulu, puis les options proposées pour ce format. Pour les formats avec perte, le débit est disponible ; pour FLAC, l’effort d’encodage est proposé. L’option **Also copy other files** conserve à côté les jaquettes, listes de lecture et autres fichiers non audio.

Quand la liste et les réglages vous conviennent, sélectionnez **Convert** et choisissez le dossier de destination. FlacCompagnon laisse la liste d’analyse intacte : convertir n’efface ni ne remplace les originaux. Choisissez un dossier vide pour une nouvelle conversion : les fichiers existants sont conservés, les noms de sortie en doublon sont refusés et les liens symboliques ne permettent pas de sortir du dossier de destination. **Also copy other files** conserve également les sorties existantes et évite de recopier le dossier de sortie lorsqu’il se trouve dans le dossier source.

## Écouter une piste

La barre inférieure est le lecteur. Cliquez sur le bouton lecture d’une ligne, ou sélectionnez des pistes puis utilisez le bouton principal de la barre.

La file de lecture est fixée au lancement : plusieurs pistes sélectionnées jouent dans l’ordre affiché ; une seule sélection démarre à cette piste puis poursuit la liste ; aucune sélection démarre en haut de la liste. Modifier la sélection pendant la lecture ne change pas la file en cours. Les boutons précédent et suivant suivent cette file. Le curseur permet de régler le volume ; la barre de progression permet de se déplacer dans le morceau.

Cette écoute est particulièrement utile après une alerte : repérez le passage indiqué par l’analyse, puis comparez-le à ce que vous entendez réellement.

## Rechercher dans une bibliothèque

Le champ **Filter…**, en haut à gauche, réduit la liste pendant que vous écrivez. Il cherche dans le nom du fichier, les informations affichées par l’analyse et les tags déjà lus : artiste, album, titre, genre, numéro de catalogue, etc.

Saisissez plusieurs mots pour garder les pistes qui contiennent chacun de ces mots dans une même information. Effacez le champ pour retrouver toute la liste. Ce filtre n’enlève aucun fichier de vos exports ou de votre rapport ; il agit seulement sur l’affichage et sur la file de lecture.

## Enregistrer et reprendre plus tard

Utilisez **Save…** pour enregistrer un rapport. Le JSON conserve les résultats d’analyse et l’ordre affiché des pistes ; il peut être rouvert par glisser-déposer dans FlacCompagnon sans relancer l’analyse.

Pour les traitements automatisés, consultez aussi [Utiliser la CLI](cli.md). Elle permet les mêmes analyses depuis un terminal et peut écrire ce même format JSON.

Les rapports importés doivent être des fichiers ordinaires de 64 Mio au maximum. Les exports de rapports, playlists et images remplacent leur destination de façon atomique et refusent les liens symboliques.

Si des pistes sont introuvables, utilisez le contrôle de présence, puis le bouton de dossier **Locate** qui apparaît, pour chercher dans un autre dossier. La recherche compare les noms de fichiers et les suffixes de dossiers, pas les empreintes audio : les correspondances à égalité restent introuvables. Réanalysez les fichiers retrouvés avant de vous fier à leurs anciennes mesures.
