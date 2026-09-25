# Lossy-transcode detection

The `Transcoded` finding looks for a codec quantization lattice left in PCM after a lossy AAC or MP3 source is decoded and then stored in a lossless container. It does not rely on a dark spectrum or a brick-wall cutoff. Those features occur in legitimate recordings often enough that they are shown as measurements, not used as a transcoding verdict.

## Principle

Lossy transform codecs quantize transform coefficients. Decoding produces PCM, but recomputing the codec analysis transform at the encoder's alignment can reveal that the coefficients remain unusually close to permitted quantization steps. The detector tests that proximity statistically over selected energetic frames and reports a lattice likelihood. The score is evidence for the tested model, not a probability that the file is lossy and not proof of origin.

AAC is tested from PCM with the codec's 2048-sample long windows and 256-sample short windows. The search considers the four AAC window shapes and all 1024 sample alignments, because a one-sample alignment error destroys the effect. MP3 needs its hybrid analysis chain: a 512-tap polyphase filterbank, 32 subbands, 18-point MDCTs and alias-reduction butterflies. It searches the 576 granule alignments. Stereo material is also examined in left/right and mid/side representations so ordinary joint stereo does not hide evidence.

The transform band layouts and windows come from the relevant codec standards. The implementation is written independently from those specifications and evaluates only the decoded samples supplied by FlacCompagnon.

## Result and applicability

The lattice result is included in the detection detail even when no finding occurs. This distinguishes “measured with no significant lattice evidence” from “the search did not complete.” A detected score creates the `Transcoded` finding only at the tabulated sample rates of 32, 44.1 and 48 kHz. At other rates, including higher rates the detail says that the lattice search is not applicable; it does not claim the source is lossless.

The authenticity summary becomes `Flagged` if this or either independent authenticity detection is positive. `Clean` means no completed detector raised a finding. It is not a certificate of recording history.

## Limits

The statistic assumes that, after the appropriate transform and scaling, uncompressed material does not concentrate on a codec grid as strongly as a matching transcode. Tonal, very quiet, processed or unusual signals can violate that model. Encoding, resampling, mixing, gain changes and re-encoding can weaken or erase a lattice. A codec or configuration outside the implemented analysis can also be missed.

Calibration remains preliminary: a pure lossless 24-bit sine can still cross the AAC threshold.

The score must therefore be read with the reported applicability and any skip reason. It should not be converted into a percentage or used alone to make a provenance claim.

## Tests

The core suite tests transform construction, grids, thresholding, alignment selection and end-to-end classification. Run:

```sh
cargo test -p flaccompagnon-core transcode
cargo test -p flaccompagnon-core analysis::detections
```

A manual AAC fixture can be made with FFmpeg by encoding a WAV to AAC and decoding it back to FLAC. The resulting fixture is useful for checking the user flow; its outcome can vary with the installed encoder and programme material:

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.2*sin(2*PI*997*t)+0.1*sin(2*PI*3401*t):s=48000:d=60' -c:a pcm_s24le source.wav
ffmpeg -n -i source.wav -c:a aac -b:a 128k encoded.m4a
ffmpeg -n -i encoded.m4a -c:a flac aac-roundtrip.flac
```
