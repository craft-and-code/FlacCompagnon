# Empreintes de fichiers : MD5 et CRC32

FlacCompagnon calcule un MD5 et un CRC32 sur tous les octets du fichier. Ils permettent de comparer des copies ou d’identifier un fichier inchangé après un renommage ou un déplacement. Le chemin ne participe pas au calcul.

## Ce qui est inclus

Tout le fichier contribue : audio compressé, en-têtes, tags, images et bourrage. Modifier un titre ou une pochette peut changer les empreintes sans modifier le son décodé. Réencoder le même son peut également les changer.

Le [MD5 audio FLAC STREAMINFO](flac-md5.md) est différent : il porte sur les échantillons audio non encodés. Il survit normalement aux modifications de tags et au réencodage sans perte d’un même flux d’échantillons. Une signature absente ou non vérifiée ne doit pas être considérée comme une identité audio vérifiée.

| Identifiant             | Survit à un déplacement simple | Survit aux modifications de tags | Signification                                |
| ----------------------- | ------------------------------ | -------------------------------- | -------------------------------------------- |
| MD5 / CRC32 du fichier  | Oui                            | Généralement non                 | Octets du fichier complet                    |
| MD5 audio FLAC vérifié  | Oui                            | Oui                              | Octets des échantillons audio décodés        |
| Identifiant MusicBrainz | Si les tags sont conservés     | Si l’identifiant est conservé    | Entité du catalogue, pas identité des octets |

## Retrouver un fichier depuis un rapport

Le JSON conserve les chemins et les empreintes. Un logiciel peut rechercher les empreintes correspondantes lorsque les chemins ont changé, mais elles ne localisent pas automatiquement le fichier. Des copies identiques partagent les mêmes empreintes ; plusieurs correspondances sont possibles. Il faut conserver cette ambiguïté plutôt que choisir un chemin arbitraire.

Les identifiants MusicBrainz complètent ces mesures lorsqu’ils sont fournis par le catalogue ou les tags : ils décrivent des enregistrements ou des éditions, et peuvent être partagés par des fichiers dont le mastering, l’encodage ou le contenu diffèrent. Ils ne remplacent pas une comparaison des octets.

## Limites

MD5 et CRC32 sont des sommes de contrôle pratiques pour les changements accidentels, pas des preuves sécurisées d’authenticité. Des collisions sont possibles, particulièrement avec CRC32 ; MD5 ne résiste pas aux collisions délibérées. Ce ne sont pas des empreintes perceptuelles : elles ne reconnaissent pas une chanson entre plusieurs encodages ou montages.

## Vérifier manuellement

Sur macOS :

```sh
md5 "track.flac"
```

Sur Linux :

```sh
md5sum "track.flac"
```

Dans PowerShell sur Windows :

```powershell
Get-FileHash "track.flac" -Algorithm MD5
```

Comparez la valeur hexadécimale avec le MD5 du fichier dans le rapport, sans tenir compte de la casse. Copiez le fichier dans un autre dossier : le résultat doit rester identique. Modifiez ensuite un tag sur la copie : le son peut rester identique alors que le MD5 du fichier change.
