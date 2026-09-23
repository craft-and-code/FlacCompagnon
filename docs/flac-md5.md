# FLAC MD5 integrity verification

The FLAC MD5 status checks whether decoded audio matches the signature stored in the FLAC `STREAMINFO` metadata block. It is an integrity check, not an analysis of recording quality, losslessness or provenance.

## Calculation

FLAC stores an MD5 of unencoded audio samples, not an MD5 of the `.flac` file bytes. During FlacCompagnon's fused FLAC decode pass, the app receives raw decoded integer samples and hashes the exact byte sequence required by the format: interleaved channels, each signed two's-complement sample encoded as `ceil(bits per sample / 8)` little-endian bytes. This avoids a float round trip and makes the successful comparison equivalent to the audio check performed by `flac -t`.

The same pass feeds the normal analysis, so MD5 verification does not require a second decode. Audio tags, cover art and container-byte changes do not enter this digest.

## Statuses

| Status                     | Meaning                                                                                            |
| -------------------------- | -------------------------------------------------------------------------------------------------- |
| `MD5 OK`                   | A stored signature was present and matched the decoded audio                                       |
| `MD5 MISMATCH`             | A stored signature did not match decoded audio; corruption or a non-conforming encoder is possible |
| `No MD5 signature`         | STREAMINFO contains the all-zero signature, so there is nothing to compare                         |
| `MD5 present (unverified)` | A signature exists but verification was disabled for this analysis                                 |
| `MD5 check error`          | The file could not be fully decoded for the check                                                  |

A mismatch concerns audio integrity. A file can have `MD5 OK` while its file-byte MD5 or CRC changes after retagging, and a file-byte checksum can match while its embedded FLAC signature is absent. FlacCompagnon exposes file fingerprints separately because the questions differ.

## Limits and tests

The check applies only to FLAC. It authenticates neither the recording history nor losslessness: a perfectly intact FLAC can still contain a lossy transcode, an upsampled source or deliberately clipped material.

```sh
cargo test -p flaccompagnon-core decode::flac
cargo test -p flaccompagnon-core flac_md5
```

For a manual independent comparison, run the reference FLAC verifier against a test file and compare its result with the app:

```sh
flac -t /path/to/file.flac
```

Reference: [RFC 9639, FLAC frame and sample representation](https://www.rfc-editor.org/rfc/rfc9639.html).
