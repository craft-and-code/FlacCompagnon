# File fingerprints: MD5 and CRC32

FlacCompagnon computes an MD5 and a CRC32 over the complete file bytes. These are useful for comparing copies or identifying an unchanged file after a rename or move. The path is not part of either calculation.

## What is included

Everything in the file contributes: compressed audio, headers, tags, artwork and padding. Updating a title or cover can change both fingerprints without changing the decoded sound. Re-encoding the same audio can also change them.

The [FLAC STREAMINFO MD5](flac-md5.md) is different: it covers unencoded audio samples. It normally survives retagging and lossless re-encoding of the same sample stream. A missing or unverified stored signature must not be treated as verified audio identity.

| Identifier              | Survives a simple move | Survives tag edits     | Meaning                             |
| ----------------------- | ---------------------- | ---------------------- | ----------------------------------- |
| File MD5 / CRC32        | Yes                    | Generally no           | Whole-file bytes                    |
| Verified FLAC audio MD5 | Yes                    | Yes                    | Decoded audio sample bytes          |
| MusicBrainz ID          | If tags are preserved  | If the ID is preserved | A catalog entity, not byte identity |

## Matching a saved report

The JSON report retains paths and the file fingerprints. A consumer can search for matching fingerprints when paths have moved, but the fingerprints do not locate a file automatically. Identical copies have identical hashes; there may be several valid matches. Preserve ambiguity instead of choosing an arbitrary path.

MusicBrainz IDs complement these measurements when supplied by the catalog or tags: they describe recordings or releases, and can be shared by files with different mastering, encoding or content. They should not replace byte comparison.

## Limits

MD5 and CRC32 are practical checksums for accidental changes, not secure authenticity proofs. Collisions are possible, especially for CRC32; MD5 is unsuitable against deliberate collision attacks. These are not perceptual fingerprints and cannot recognize a song across different encodings or edits.

## Check manually

On macOS:

```sh
md5 "track.flac"
```

On Linux:

```sh
md5sum "track.flac"
```

In Windows PowerShell:

```powershell
Get-FileHash "track.flac" -Algorithm MD5
```

Compare the hexadecimal digest with the file MD5 in the report, ignoring letter case. Copy the file to another folder and repeat: the digest should be unchanged. Then edit a tag on the copy and compare again; its audio can remain identical while the whole-file MD5 changes.
