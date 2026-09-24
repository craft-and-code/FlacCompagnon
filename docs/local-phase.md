# Local and frequency-band phase correlation

These measurements reveal stereo opposition that the [whole-track polarity measurement](stereo-polarity.md) can dilute: a short opposed passage, or opposed treble beneath stronger aligned bass. They describe decoded stereo audio and help locate passages to audition in mono. They do not establish an encoding history, an absolute polarity, or an unintended defect.

## Results in the app

Both columns contain only a signed correlation coefficient, rounded to three decimals:

| Column      | Value                                                              | Details on hover                                                                         |
| ----------- | ------------------------------------------------------------------ | ---------------------------------------------------------------------------------------- |
| Local phase | Lowest correlation among eligible broadband windows                | Aggregate correlation, minimum window start, opposed-window share, eligible window count |
| Band phase  | Lowest correlation among eligible windows in any of the four bands | The same evidence separately for each band                                               |

Both sort numerically from −1 to +1 and are searchable by the displayed value and column name. Missing measurements sort last in either direction. A dash means no usable measurement, while `0.000` is a measured coefficient rounded to display precision.

For either result:

- **+1** means the channels are proportional with matching polarity in the measured range. Their levels need not match.
- **0** means no zero-lag correlation. This includes a quarter-cycle phase difference on a pure tone; it does not establish that the channels are unrelated.
- **−1** means the channels are proportional with opposite polarity in the measured range. Unequal gains can still prevent total cancellation when summed to mono.

An opposed window is defined here as a coefficient **at or below −0.5**. This is a project reporting threshold for substantially negative correlation, not an industry standard, an audibility threshold or a polarity-inversion verdict. No new authenticity flag or automatic defect label is produced. The existing whole-track polarity indication keeps its own method and threshold.

## Time resolution and frequency ranges

The meter accepts exactly two channels at sample rates from **8 to 768 kHz**, including stereo PCM decoded from DSD when decoding is enabled. Mono and multichannel files are not reduced to an arbitrary channel pair.

The transform size is `N = next_power_of_two(ceil(sample_rate / 5))`. Thus each window is at least 200 ms and less than 400 ms long. The hop is `N / 2`, giving 50% overlap. At 48 kHz, this is **341.333 ms**, with a **170.667 ms** hop. At 44.1 kHz, the window is **371.519 ms**. The actual window and hop are saved in the report.

| Range          | Nominal edges |
| -------------- | ------------- |
| Broadband      | 20 Hz–20 kHz  |
| Bass           | 20–200 Hz     |
| Midrange       | 200 Hz–2 kHz  |
| Upper midrange | 2–6 kHz       |
| Treble         | 6–20 kHz      |

Upper limits are capped at Nyquist (`sample_rate / 2`). Lower edges are inclusive, upper edges exclusive. DC and the Nyquist bin are excluded. Bands entirely above Nyquist have no reading. FFT bins belong to the range containing their center frequency; the bin spacing is `sample_rate / N`.

Only complete windows are examined. There is no zero padding of the trailing partial window. A file shorter than one window has no result. Window starts are measured from the first decoded frame and include silent introductions. They identify the analysis window, not the exact beginning of a phase change. The minimum's window end is its start plus `window_secs`; when minima tie at stored precision, the first such window is retained.

## Calculation

For each channel independently, calculate a periodic Hann window and its weighted mean:

```text
w[n] = 0.5 − 0.5 cos(2πn/N),  n = 0 … N−1
μ = Σ(w[n] × x[n]) / Σw[n]
X[k] = DFT(w[n] × (x[n] − μ))
```

The periodic Hann convention follows the [MathWorks window documentation](https://www.mathworks.com/help/signal/ref/hann.html). Its weighting reduces leakage at window boundaries. Removing the mean prevents a constant channel bias from dominating the phase reading; [DC offset](dc-offset.md) remains separately available.

Let `L[k]` and `R[k]` be the two complex spectra. For each frequency range, sum the positive-frequency bins in that range:

```text
q = 2 / (N × Σw[n]²)
E_L = q × Σ |L[k]|²
E_R = q × Σ |R[k]|²
C   = q × Σ Re(L[k] × conjugate(R[k]))
correlation = C / sqrt(E_L × E_R)
```

The factor `q` comes from Parseval's identity, the two symmetric frequency halves of a real signal, and the window's mean-square gain. It makes the energy gate comparable between transform sizes. It cancels in the correlation itself. Results are clamped to −1 through +1 to contain floating-point roundoff.

A window is eligible for a particular range only when **both** `E_L` and `E_R` exceed `10⁻⁶`, equivalent to a band RMS above −60 dBFS per channel. Each range has its own gate. A strong bass component cannot make an otherwise silent treble band eligible. The floor is a project choice to suppress meaningless ratios in silence and very weak leakage; it is not loudness weighting.

For each range, the report retains:

- Correlation calculated from the **sum of eligible-window energies**. It is not an arithmetic average of individual coefficients.
- Minimum local correlation and the start of the corresponding window.
- Fraction of eligible windows with a coefficient at or below −0.5.
- Eligible window count, alongside the total complete-window count for coverage.

The opposed fraction counts **overlapping eligible windows**, not samples or seconds. It must not be presented as the percentage of the track affected. Silence and one-sided content do not contribute to the denominator. At most five summaries are retained, so memory use and report size stay bounded with track duration.

## Interpretation limits

- This is **zero-lag correlation**, not a phase angle in degrees and not magnitude-squared coherence. An averaged coherence estimate and the cross-spectrum angle answer different questions; see the [MathWorks discussion of cross spectrum and coherence](https://www.mathworks.com/help/signal/ug/cross-spectrum-and-magnitude-squared-coherence.html). FlacCompagnon uses the real cross spectrum here to quantify agreement or opposition within each range.
- Broadband and band results are energy-weighted. Multiple components can cancel each other's correlation inside the same band. This is a four-band summary, not a phase-versus-frequency curve.
- Intentional stereo widening, ambience, delays and microphone spacing can produce negative correlation. Listening and knowledge of the source are required before changing the audio.
- A brief event shorter than a window can be diluted. Hann weighting also reduces the influence of samples near each window edge. The method does not promise sample-accurate event detection.
- Windowing reduces spectral leakage without eliminating it. A tone close to a band boundary can contribute to both neighbouring bands; the edges are analysis bins, not ideal physical crossover filters. Subsonic and ultrasonic content can also leak into the measured range.
- Low-frequency estimates contain relatively few cycles. An isolated transient or a very short excerpt is less representative than sustained programme material.
- The absolute energy gate makes coverage depend on gain. Raising a very quiet file can make previously unavailable ranges measurable. A missing band is not proof of perfect mono compatibility.
- Global polarity uses whole-stream samples, while this measurement removes DC, limits the frequency range, windows the audio and gates each range. Their aggregate coefficients need not be equal.
- Invalid frames or non-finite samples withhold the entire measurement. Failed/skipped decoding and older saved reports also have no reading. Float samples above full scale are measured without clipping them first.

## Saved reports

JSON adds an optional `local_phase` object with `window_secs`, `hop_secs`, `analyzed_windows`, `broadband`, and four `bands`. Each band contains its effective `low_hz`, `high_hz`, and an optional `summary`. Summaries contain `correlation`, `minimum_correlation`, `minimum_start_secs`, `opposed_fraction` and `eligible_windows`. A band with `high_hz <= low_hz` is above Nyquist and has no summary.

CSV adds `phase_window_s`, `phase_hop_s`, and `phase_analyzed_windows`, then five numeric fields for each range:

| Prefix              | Range          |
| ------------------- | -------------- |
| `local_phase_`      | Broadband      |
| `phase_20_200_`     | Bass           |
| `phase_200_2000_`   | Midrange       |
| `phase_2000_6000_`  | Upper midrange |
| `phase_6000_20000_` | Treble         |

The suffixes are `correlation`, `minimum`, `minimum_start_s`, `opposed_fraction`, and `eligible_windows`. The frequency labels are nominal; use the row's `sample_rate` to determine Nyquist. Fractions remain in 0–1 and timestamps in seconds. Missing values are blank, measured zeros remain zero. File fingerprints remain the final two columns. Existing JSON reports load with `local_phase: null` and require reanalysis to obtain these values.

## Automated verification

```sh
cargo test -p flaccompagnon-core local_phase
node --test tests/analysis-cells.test.mjs tests/search.test.mjs
```

Unit tests live in `core/tests/unit/analysis/local_phase.rs`; the independent expected values come from sine phase identities and sums of orthogonal tone energies. They cover 0°, 60°, 90° and 180°, unequal gains, opposite treble under aligned bass, a short opposed passage with a known position, DC rejection, the RMS gate, invalid input, incomplete windows, supported rates and Nyquist limits. `core/tests/local_phase.rs` passes an independently generated 24-bit WAV through the complete decoder and verifies the reported timing, bands, CSV and JSON, including loading an older report. Frontend tests cover signed sorting, values, missing readings, tooltips and search.

## Manual fixtures with FFmpeg or Audacity

Run these from a folder where you want to keep the test audio. `-n` prevents overwriting an existing file. All three signals stay comfortably below clipping.

### 1. Brief opposition hidden by the whole-track average

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.2*sin(2*PI*1000*t)|if(between(t\,2\,3)\,-1\,1)*0.2*sin(2*PI*1000*t):s=48000:d=6' -c:a pcm_s24le phase-local.wav
```

Expect **Local phase ≈ −1.000**, with its minimum window contained within 2–3 s and a nonzero opposed-window share. The midrange band is opposed there too. The whole-track correlation remains positive, approximately +0.667. In Audacity, generate a stereo tone, split the channels and invert only one channel between 2 and 3 s for an equivalent test. Listen to a mono mix to hear the cancellation in that passage.

### 2. Opposed treble under aligned bass

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.2*sin(2*PI*100*t)+0.1*sin(2*PI*9000*t)|0.2*sin(2*PI*100*t)-0.1*sin(2*PI*9000*t):s=48000:d=5' -c:a pcm_s24le phase-bands.wav
```

Expect **Local phase ≈ +0.600** and **Band phase ≈ −1.000**. On hover, bass is about +1, treble about −1 with 100% opposed windows, and the two unused middle bands should be unavailable. The analytical broadband value is `(0.2² − 0.1²) / (0.2² + 0.1²) = 0.6`. On summing the channels, the 9 kHz component cancels while the bass remains.

### 3. Quadrature is a measured zero

```sh
ffmpeg -n -f lavfi -i 'aevalsrc=0.2*sin(2*PI*1000*t)|0.2*cos(2*PI*1000*t):s=48000:d=5' -c:a pcm_s24le phase-quadrature.wav
```

Expect both columns near **0.000** and an opposed-window share of 0%. The midrange band has a measurement; the other bands are unavailable. Changing the cosine to the same sine as the left channel produces +1.000, and negating that sine produces −1.000. These checks distinguish real zero correlation from missing data.
