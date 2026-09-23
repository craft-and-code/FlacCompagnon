# Integer bit-depth detection

The app gives a binary Upscaled flag. A negative flag means that neither of
the signatures below was established; it is not a certificate of recording
history. Detection results are binary. Unavailable checks are
explained in the detection detail; file read failures remain explicit errors.

## Exact unused bits

Every decoded integer sample, on every channel and through the end of the file,
contributes to a bitwise OR. Trailing zero bits identify unused precision exactly.
For example, widening 16-bit PCM to 24 bits by multiplication by 256 leaves eight
zero low bits. Negative samples are handled as two's-complement integers.

The existing silence rule is deliberately preserved: an entirely zero signal
reports one effective bit and is flagged when the declared width is greater.
One bit is the display convention, not evidence of a one-bit recording.
An empty or broken stream is a decoding error, not digital silence.

Integer values must remain integers for this calculation, including 32-bit PCM.
Float samples still feed spectral analysis, but must not be used to reconstruct
the integer low bits. Floating-point sources have no integer depth measurement.

## A grid surviving low-level export noise

An export can add a small residual to a widened signal. Then the OR correctly
reports all occupied bits, including that residual, but misses the lower-depth
grid. For a candidate step `S = 2^(declared_bits - candidate_bits)`, the second
check measures the distance to the nearest multiple of S using integer arithmetic.

Requirements are deliberately conservative:

- Candidate source widths are at least 8 bits; the gap is at least 6 bits.
- Every sample must be within `S/16` of the grid. A single outside sample,
  including in the last packet, permanently rejects that candidate.
- At least three disjoint windows of 4096 samples per nonzero channel must each
  span at least 64 grid steps and populate at least 32 of the 64 coarse-code
  residues. Few-level signals, DC plus tiny noise, tiny signals, sparse impulses,
  and silence cannot supply this evidence.
- Each channel is measured separately; the file takes the largest required
  depth. A native channel cannot be outvoted by padded or silent channels.
- All samples are checked even after enough rich windows have been found.

These are engineering thresholds, not published confidence levels. The radius
is far smaller than half a grid step: with a 16-bit grid in 24-bit PCM, accepted
residues cover only 33 of 256 possible integers. Real music is not independent
uniform noise, so this ratio is not a false-positive probability.

The supplied Audacity export contains 16,558,080 samples. Independent FFmpeg PCM
measurement found a residual between -10 and +10 destination LSB, RMS 2.050271,
around a 16-bit grid spanning 64,972 distinct codes. All 24 bits are occupied.
142 samples exceed +/-8, which is why a strict eight-LSB bound would miss it.
The chosen +/-16 bound covers this file; the unit/integration corpus tests both
positive constructions and counterexamples rather than using this file alone.

The app displays this result as `≈16-bit`, keeps exact occupied depth in
`bit_depth_evidence.stored_bits`, and explains the residual in the detection
detail. Saved JSON retains both values. CSV exports also keep `bit_depth_method`
and `stored_bits`, so estimated depth can be distinguished without changing the
numeric `real_bit_depth` column.
Old JSON reports without the new evidence field still load.

## Limits and rejected approaches

An intentionally generated grid-like signal, or 16-bit-grid music mixed with a
genuine signal weaker than the accepted residual, can be indistinguishable from
an export with dither. Hence the grid result is an estimate, not an assertion
that the residual must be dither. Stronger noise, gain changes, resampling, or
other processing can erase the grid and lead to a negative result.

Do not infer source bit depth from a -98 dBFS noise floor. The familiar
`6.02*N + 1.76 dB` equation describes an ideal quantizer driven by a full-scale
sine wave under specific noise/bandwidth assumptions. Acoustic noise, gain,
dither, FFT bandwidth, and window normalization defeat a universal threshold.
A noisy native 24-bit recording is not thereby upscaled. A low spectral floor
also does not certify a native master. The PDF's early exit after 100,000
samples is rejected because a padded intro says nothing about the rest.

## Sources and reproducibility

Validation on 2026-09-23: all 296 Rust tests (including 8 new cross-module
upscaling tests and the expanded integer-decoding tests), TypeScript checking,
frontend production build, 12 frontend regression tests, and Clippy across all targets passed. A release-mode
probe through the app's analysis code returned:

| Supplied file | Declared | Effective | Exact occupied bits | Upscaled |
| --- | --- | --- | --- | --- |
| `test_upscaled.flac` | 24 | approximately 16 | 24 | yes |
| `11 You Can Make Your Own Music.flac` | 16 | 1 (silence convention) | 1 | yes |

These results validate the supplied cases and synthetic corpus, not a universal
error rate across all recordings. Existing displayed or imported results must
be reanalyzed with a freshly built app; an installed older binary is unchanged.

The supplied 13-track No More Tears album (24-bit / 96 kHz) was also checked
through the complete file-analysis pipeline. Every track reports 24 effective
bits, Clean, no detection flags, and no read error. Unavailable checks remain
explicit in the detail and do not change the binary summary. Saved reports
with unsupported detection statuses require reanalysis; obsolete results are
not imported or converted.

- [FLAC specification, RFC 9639, wasted bits](https://www.rfc-editor.org/rfc/rfc9639.html#section-9.2.2): exact low-bit padding and restoration; FLAC supports 4–32 bits.
- [Audacity Dither manual](https://manual.audacityteam.org/man/dither.html): float processing and export dither. Its illustrative +/-3 figure does not bound the supplied shaped export, measured at +/-10.
- [Analog Devices: data-converter noise budget](https://www.analog.com/en/resources/technical-articles/selecting-the-best-data-converter-for-a-given-noise-budget-part-3.html): conditions behind SNR and ENOB, distinct from storage width.
- [Lipshitz, Wannamaker and Vanderkooy, Quantization and Dither: A Theoretical Survey (1992)](https://hajim.rochester.edu/ece/sites/zduan/teaching/ece472/reading/Lipshitz_1992.pdf): dither properties and their statistical assumptions.

Implementation is derived from integer arithmetic and these principles, not
another detector's source. Run `cargo test -p flaccompagnon-core --offline` for
the synthetic regression corpus, and the following read-only probe for a real file:

```sh
cargo run --release -p flaccompagnon-core --example bitdepth -- /path/to/track.flac
```
