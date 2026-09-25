# Vérification de la signature MD5 FLAC

L’état MD5 FLAC compare l’audio décodé à la signature stockée dans le bloc de métadonnées `STREAMINFO`. Il vérifie l’intégrité, pas la qualité, le caractère sans perte de la source ou sa provenance.

## Calcul

La signature FLAC porte sur les échantillons audio non encodés, pas sur les octets du fichier `.flac`. Pendant le décodage, FlacCompagnon reçoit les entiers bruts et hache exactement la séquence définie par le format : canaux entrelacés, échantillons signés en complément à deux, chacun codé sur `ceil(bits per sample / 8)` octets en petit-boutiste.

Cela évite un aller-retour en flottant et rend la comparaison équivalente au contrôle audio de `flac -t`. La même passe alimente les analyses courantes ; aucune seconde passe de décodage n’est nécessaire pour le MD5. Tags, images et modifications des octets du conteneur n’entrent pas dans cette signature.

## États

| État                       | Signification                                                          |
| -------------------------- | ---------------------------------------------------------------------- |
| `MD5 OK`                   | Signature présente et identique au calcul sur l’audio décodé           |
| `MD5 MISMATCH`             | Signature différente ; corruption ou encodeur non conforme possibles   |
| `No MD5 signature`         | Signature entièrement nulle dans STREAMINFO, sans comparaison possible |
| `MD5 present (unverified)` | Signature présente, mais vérification désactivée                       |
| `MD5 check error`          | Décodage complet impossible pour cette vérification                    |

Un fichier peut rester `MD5 OK` alors que son MD5 ou CRC32 **de fichier** change après retagage. Inversement, une somme de contrôle du fichier peut être identique sur deux copies alors que la signature audio FLAC est absente. Les [empreintes de fichier](fingerprints.md) répondent à une autre question.

## Limites et vérification

Ce contrôle concerne seulement le FLAC. Un FLAC intact peut contenir une source avec perte, suréchantillonnée ou écrêtée. Le MD5 ne certifie pas l’historique de l’enregistrement.

```sh
cargo test -p flaccompagnon-core decode::flac
cargo test -p flaccompagnon-core flac_md5
flac -t /path/to/file.flac
```

Comparez le résultat de l’outil de référence `flac -t` à celui de l’application.

Référence : [RFC 9639, format FLAC](https://www.rfc-editor.org/rfc/rfc9639.html).
