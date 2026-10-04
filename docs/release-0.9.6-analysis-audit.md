# 0.9.6 audio measurement audit

This follow-up, performed on 5 October 2026, reviews Local phase, Band phase, Balance, HF Stereo, DC, Impulses, Dropouts, integrated LUFS, LUFS-M Max, LUFS-S Max and LRA. It combines standards and scientific literature, practitioner discussions, independent signal fixtures, executable reference comparisons and a CPU benchmark. The earlier [security and maintenance audit](maintenance-audit.md) remains the record of local-security and dependency work.

Transcoded, Upscaling and Upsampling remain excluded. Their implementations, thresholds, dedicated tests and documentation are unchanged. No new dependency, payload field, detection label or external implementation source is introduced.

## Measurement choices and confirmed corrections

| Measurement     | Definition retained                                                                              | Correction or verification                                                                                                                                                                                                                                                       |
| --------------- | ------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Local phase     | Signed zero-lag correlation in complete Hann windows, with a separate audible-energy gate        | Anchoring the weighted mean at the smallest-magnitude sample prevents large constant DC from manufacturing spectral energy without losing ordinary samples after a first-sample outlier. Analytical phase, Parseval normalization, aggregation and malformed frames are covered. |
| Band phase      | The same signed correlation in four fixed frequency ranges                                       | Analytical Hann leakage tests specify the band-edge convention. The spectral scan stops at 20 kHz or Nyquist; unavailable bands remain unavailable.                                                                                                                              |
| Balance         | Whole-file raw L/R RMS level difference, including DC                                            | Independently generated float WAVs verify unequal levels, silence, DC and duration weighting. Malformed trailing frames can no longer publish a valid prefix as a whole-file measurement.                                                                                        |
| HF Stereo       | Matched filtered Mid/Side powers in the high and reference bands                                 | Initial-value priming removes the artificial HF transient caused by a constant channel bias at the beginning of an excerpt. Genuine later steps and passbands remain unchanged.                                                                                                  |
| DC              | Signed per-channel arithmetic mean, reported relative to full scale                              | Existing compensated summation is shared with the discontinuity meters. Analytical cancellation and decoded float-WAV checks cover extreme finite inputs.                                                                                                                        |
| Impulses        | Two opposing, unusually large edges, at most 0.5 ms apart, with quiet 5 ms context on both sides | The final scan now uses the actual pulse width, finding short pulses near the end when their context is complete. Reused compensated prefix sums retain quiet context after a large transient.                                                                                   |
| Dropouts        | Exact-zero gaps lasting 2–250 ms, bounded by abrupt edges and sustained, comparable context      | Compensated additions and removals preserve rolling energy after an extreme finite float sample leaves the context. A decoded float-WAV regression verifies the later gap is still found.                                                                                        |
| Integrated LUFS | K-weighted 400 ms powers, absolute and relative gating                                           | Filter retuning accounts for bilinear frequency warping at low sample rates and preserves the published RLB numerator. Compensated window sums recover quiet passages after extreme float levels. Blocks below the fixed absolute gate are discarded as soon as they are known.  |
| LUFS-M Max      | Ungated maximum over complete 400 ms windows                                                     | Shares the corrected K-weighting; peak hold checks every decoded frame, rather than only display updates.                                                                                                                                                                        |
| LUFS-S Max      | Ungated maximum over complete 3 s windows                                                        | Shares the corrected K-weighting; analysis-only LRA tail silence cannot move the real-audio maximum.                                                                                                                                                                             |
| LRA             | Gated distribution of overlapping 3 s powers; 95th minus 10th percentile                         | Reuses its history for two order statistics instead of allocating and sorting a second vector. The original percentile convention is preserved. Tail padding rounds upward at odd sample rates so it remains at least 1.5 s.                                                     |

Whole-file polarity also uses separate energy square roots, avoiding overflow/underflow in their product and retaining gain invariance for very quiet finite float audio. Shared stereo accumulation now counts exactly equal decoded samples; an absolute `1e-9` tolerance could previously mistake distinct quiet channels for dual mono. Removing an absolute energy epsilon also preserves the existing −60 dB relative-difference rule under common gain. A shared validity flag withholds whole-file stereo results after malformed input without changing the independent authenticity paths.

## Standards and alternatives

The loudness definitions follow [ITU-R BS.1770-5, Annex 1](https://www.itu.int/dms_pubrec/itu-r/rec/bs/R-REC-BS.1770-5-202311-I!!PDF-E.pdf), [EBU Tech 3341](https://tech.ebu.ch/docs/tech/tech3341.pdf) and [EBU Tech 3342](https://tech.ebu.ch/docs/tech/tech3342.pdf). The ITU coefficients are preserved at their 48 kHz reference rate. Other supported rates use a pole-frequency prewarp derived from those coefficients, while keeping the RLB numerator `[1, -2, 1]`. LRA retains its distinct −20 LU relative gate, at least 10 Hz updates and file-tail treatment; M/S maxima remain ungated.

Alternative perceptual filters and time constants were reviewed in the scientific paper [Steinmetz and Reiss, 2021](https://csteinmetz1.github.io/pyloudnorm-eval/paper/pyloudnorm_preprint.pdf). Better prediction of subjective loudness for some isolated instruments or sound effects does not make a modified filter interchangeable with standard LUFS. Such a new perceptual measurement would need its own name, listening data and validation. The abstract of [De Man, AES 2018](https://aes.org/publications/elibrary-page/?id=19790) also supports checking filter coefficients across sample rates; its paywalled full text was not used as implementation evidence.

For phase, maximizing correlation across time lag could describe delay but hide opposition in the actual mono sum. Magnitude-squared coherence describes linear relatedness and requires averaging across segments; it does not preserve the signed interpretation of this meter. The [official coherence documentation](https://www.mathworks.com/help/signal/ref/mscohere.html) and analytical sine identities support keeping the current signed measurement. Fixed frequency bands retain explicit Hann leakage and bin boundaries rather than claiming auditory-band separation.

Balance remains a level comparison, rather than a recommendation to centre every mix. HF Stereo measures filtered Mid/Side energy, which is affected by panning as well as phase; it cannot prove intensity-stereo encoding. The [ITU's audio-coding tutorial](https://www.itu.int/dms_pub/itu-t/opb/tut/T-TUT-IPTV-2009-MCTA-PDF-E.pdf) distinguishes those coding tools. Research on [perceived source width](https://pmc.ncbi.nlm.nih.gov/articles/PMC3566657/) concerns correlation at listeners' ears and does not calibrate a decoded-file L/R defect threshold.

The phase, HF Stereo and discontinuity pages document intentional stereo effects, transients and mutes that can produce the same evidence as an unwanted artifact. A [firsthand sampled-piano discussion](https://forum.soundonsound.com/phpbb/viewtopic.php?embed=true&t=79368) illustrates why a negative meter reading still needs auditioning. These discussions inform interpretation, not numerical ground truth. For impulses, [auditory/wavelet listening-test research](https://link.springer.com/article/10.1186/s13636-024-00389-9), [music-context click classification](https://archives.ismir.net/ismir2021/paper/000095.pdf) and [sparse linear prediction](https://ftp.esat.kuleuven.be/pub/stadius/vanwaterschoot/reports/dufera2019.pdf) suggest alternatives requiring a labelled corpus, perceptual validation or more fitting work. None establishes that replacing this conservative cue would improve its musical false-positive rate.

## Public-tool comparison

A release comparison inspected the public definitions and presentation in [iZotope RX Waveform Statistics](https://downloads.izotope.com/docs/rx6/57-waveform-stats/index.html), [Youlean's measurement guide](https://docs.youlean.co/youlean-loudness-meter/getting-started/measurements-explained) and the [NUGEN Monofilter manual](https://nugenaudio.com/files/manuals/Monofilter4%20Manual.pdf). Shared standard names and M/S window lengths are expected. FlacCompagnon's documented fixed-band summaries, separate missing values and conservative event cues remain its own design. No third-party source was fetched or used as a coding model. This public-documentation comparison cannot establish whether another tool shares an internal name or bug.

## CPU and memory evidence

The reproducible fixture is [measurement_bench.rs](../core/examples/measurement_bench.rs): ten seconds of precomputed stereo tones, changing level, channel bias, one pulse and one exact-zero gap. Fixture generation and file decoding are excluded; meter construction, frame processing and finalization are included. Each case warms up once and reports the median of three runs. The combined selection covers only the measurements in this audit.

```sh
cargo run -p flaccompagnon-core --release --example measurement_bench
```

The before/after comparison uses the same harness, with the previous source taken from commit `a775239d32760c24b47b7d15cdc8a39493e9d013`. Both link the core at the repository's development override of `opt-level=3` into a harness compiled with `rustc -C opt-level=3`; debug assertions remain enabled in the core. The following values are the median of three complete benchmark runs, each containing three timed repetitions per case, on macOS arm64 with Rust 1.99.0. These measurements do not include decoding, the desktop UI, authenticity searches or a representative music-library workload. The release command above can produce different absolute timings.

Time to process ten seconds of stereo audio at 48 kHz:

| Selection               | Before (ms) | After (ms) | Change |
| ----------------------- | ----------: | ---------: | -----: |
| Dispatch without meters |       2.110 |      2.225 |  +5.5% |
| Balance / shared stereo |       2.566 |      2.840 | +10.7% |
| Local / Band phase      |      17.127 |     17.930 |  +4.7% |
| HF Stereo               |      28.207 |     27.723 |  −1.7% |
| DC                      |       3.770 |      3.769 |    ≈0% |
| Impulses                |       8.070 |      6.313 | −21.8% |
| Dropouts                |       5.456 |      5.996 |  +9.9% |
| LUFS / M / S / LRA      |       6.851 |      7.747 | +13.1% |
| All selections above    |      58.880 |     59.666 |  +1.3% |

Combined selections across sample rates:

| Sample rate | Before (ms) | After (ms) | Change |
| ----------- | ----------: | ---------: | -----: |
| 44.1 kHz    |      54.075 |     53.431 |  −1.2% |
| 48 kHz      |      58.880 |     59.666 |  +1.3% |
| 96 kHz      |     116.377 |    110.967 |  −4.6% |
| 192 kHz     |     237.357 |    224.939 |  −5.2% |

Compensated rolling sums and stereo validity checks deliberately add some work. Lazy impulse prefixes avoid unnecessary context calculations, and selecting the local-phase anchor through finite IEEE magnitude ordering reduces its cost. Small percentage changes include scheduling and timer noise: the three combined 48 kHz medians ranged from 57.618–59.445 ms before and 57.705–59.954 ms after. The combined workload therefore remains similar at common rates, with a clearer reduction at 96/192 kHz. Its final 48 kHz time is about 0.6% of audio duration. This is evidence for this fixture, not a whole-application speed guarantee.

Local/band phase, HF Stereo, DC and discontinuity state are bounded by sample rate and channel count. Reusing the impulse prefix avoids repeated allocation; its compensated entries hold two numbers so that subtraction does not erase quiet context. Loudness keeps fixed 400 ms/3 s rings plus duration-dependent histories of above-gate powers. Early absolute gating removes useless silence history, but active programme history still grows at approximately 10 values per second for each distribution. The LRA order-statistic pass uses the original vector and linear selection rather than a second vector and a full sort.

## Release and verification

The Cargo workspace, all four resolved crate versions, npm manifest and lockfile, Tauri configuration, site metadata and README tag example already agree on **0.9.6**. No version was lowered or incremented beyond the requested release. Release preparation does not create a tag, push changes or publish installers.

The per-measurement pages in English and French contain formulas, interpretation, limitations, sources and targeted reproduction commands. Tests use independently derived signals and normative expectations; a regression is added for each reproduced defect.

The optional FFmpeg 9.0.2 loudness check passed for seven sample rates (8, 11.025, 16, 32, 44.1, 48 and 96 kHz) in mono and stereo. It compares sixty-second signals containing both K-filter transitions and four sustained levels. Three-decimal metadata showed differences of approximately 0.013 LU or less for integrated loudness, 0.002 LU for maximum M and 0.001 LU for maximum S; LRA agreed to the reported precision. The test acceptance limits remain 0.1 LU for I/M/S and 1 LU for LRA. Fixed steady-state tone responses at 30/100/1000/3000 Hz, across 8/11.025/48/96 kHz, also agree with independently recorded reference values within 0.001 LU. The old 8 kHz response differed by up to about 0.21 LU. Those narrow response checks are distinct from a claim of that accuracy on every programme.

The independent DC comparison also passed through WAV and FLAC encoding. The complete verification results below include the final integer-reduction optimization.

| Check                                                                               | Result                                                |
| ----------------------------------------------------------------------------------- | ----------------------------------------------------- |
| Version consistency across manifests, resolved workspace packages and site metadata | All 0.9.6; no local `v0.9.6` tag created              |
| `npx tsc --noEmit` and `npm run build`                                              | Passed                                                |
| `npm test`                                                                          | 108 passed                                            |
| AppImage Python regression suite                                                    | 6 passed                                              |
| `cargo test --workspace --locked`                                                   | 587 passed; 3 optional native/reference tests ignored |
| Optional FFmpeg loudness and DC comparisons, run separately                         | Both passed                                           |
| `cargo clippy --workspace --all-targets --locked -- -D warnings`                    | Passed                                                |
| `cargo build --workspace --locked`                                                  | Passed                                                |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --locked --no-deps`               | Passed                                                |
| Rust formatting and `git diff --check`                                              | Passed                                                |
| Markdown formatting and documentation site build                                    | Passed; 55 HTML pages and 2,819 local links checked   |

The optional native credential-store test remains skipped in this DSP pass; the earlier security audit records its separate macOS round trip. No dependency resolution changed, so the previously documented four Cargo advisory warnings still need their coordinated upstream replacements or focused backports.

The remaining accuracy boundary is explicit: synthetic fixtures and executable comparisons demonstrate the tested properties, not universal musical defect recognition or official meter certification. There is no representative independent listening corpus for estimating false positives in Impulses, Dropouts or HF Stereo. LRA on short or isolated material remains unstable, as discussed by the [EBU](https://tech.ebu.ch/news/2016/08/ebu-loudness-changes). Loudness still supports mono/stereo only until decoders provide reliable multichannel speaker roles. This pass does not run new Windows/Linux installers, native audio-device tests or systematic fuzzing.
