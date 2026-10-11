# Camera RAW audit — 2026-10-11

This audit uses the seven supplied files, one downloaded DPReview X-T5 RAF, and
74 checksum-verified CC0 samples from raw.pixls.us. The public samples cover 19
camera models and eight format families: ARW, CR3, DNG, NEF, ORF/ORI, PEF, RAF and
RW2. The complete set covers 24 models. This is a reproducible sample audit,
not certification of every camera or shooting mode.

All 82 files open: 45 develop from sensor data into full-resolution 16-bit
ProPhoto documents, and 37 import explicit 8-bit camera-JPEG fallbacks. The
public subset is 40 sensor developments and 34 fallbacks. Every native import
was checked for its dimensions, depth and embedded profile.
The three original D5100 NEFs were also rechecked: all retain the 4928 × 3264
image area and 16-bit sensor import, with orientation applied. Across both sets,
85 files open (48 sensor, 37 preview).

## Changes

- Canon CR3 now opens its camera JPEG, using the ISO BMFF JPEG track's validated
  sample tables and media offsets, or the documented PRVW container. CMT1 supplies
  orientation. Sensor CRX remains unsupported; the import explicitly reports an
  8-bit camera preview. No compressed-data signature scanning is used.
- Development memory limits account for actual allocation lifetimes and bound
  simultaneous AHD bands. The 11648 × 8736 Fujifilm images now develop under the
  default 2 GiB working-memory limit without increasing it. The caller's sensor
  storage and the eventual document are separate allocations, so this is not a
  2 GiB process-memory guarantee. Limiting AHD concurrency leaves pixels unchanged.
- Nikon lossy linearization version `0x44 0x40` uses its smaller encoded domain,
  preserving sensor ADUs. Previously, a 14-bit Z6 image decoded to at most 1023
  against a recorded black level of 1008 and developed almost entirely black.
  Synthetic tests cover both byte orders and 12-/14-bit depths; pinned real
  samples cover both depths, with additional Z7 and D780 audit files.
- Nikon maker-note compression modes 6 and 10 route to their packed 12-/14-bit
  reader despite TIFF Compression 34713. The reader checks a single strip, bit
  depth, byte order and exact row layout, including 16-byte row padding. It does
  not infer the codec from file size. Five Z6/Z7/D850 files previously fell back
  to JPEG and now develop their full sensor data.
- Nikon CropArea supplies validated sensor coordinates rather than body-specific
  assumptions. The tested Z6/D780 images crop to 6048 × 4024 and Z7 to 8256 × 5504.
  Invalid rectangles retain the full valid image area.
- Color fitting can align a camera JPEG's tighter framing with the RAW proxy.
  A centered search is bounded to 90–100% of the frame's dimensions and uses
  training cells only. It requires correlation of at least 0.9, improving by
  at least 0.05; the existing picture, matrix and held-out color gates still
  apply. The developed crop stays unchanged. This lets the tested Z7 images
  retain a fitted appearance instead of rejecting their camera previews.
- Public `Sensor` crop and CFA metadata are validated before development, including
  hostile dimensions, sample layouts, CFA periods and extreme origins.

Lossless compressed RAF support comes from the existing
[PR #2371](https://github.com/storytold/photocraft/pull/2371), included as a separate
dependency. Embedded-JPEG color fitting comes from
[PR #2135](https://github.com/storytold/photocraft/pull/2135); this audit does not
introduce measured camera color profiles.

## Coverage and remaining work

| Camera group | Modes exercised | Remaining limitation |
|---|---|---|
| Canon EOS R, R6, R5, M50, M6 Mark II | RAW/cRAW; full, cropped, burst variants | CR3 opens camera JPEG; CRX sensor decoding and burst frame selection remain open |
| Fujifilm X-T10, X-T5, GFX 100, GFX 100S, GFX 100 II | Bayer/X-Trans; uncompressed and lossless compressed; 14/16-bit; sports crop; orientation | Lossy RAF stream version 0 remains preview-only; native calibrated color profiles missing |
| Nikon Z6, Z7, Z8, D780, D850 | Packed uncompressed, lossy and lossless; 12/14-bit; D850 small/medium/large | Z8 High Efficiency and High Efficiency* remain preview-only; lossy-after-split remains open |
| Sony ILCE-7M4 | Uncompressed and lossless large/medium/small | Reduced-size M/S lossless remains preview-only |
| Panasonic S5 II, GH6, G9 | Normal, high-resolution and secondary images | Modern/vendor-compressed RW2 remains preview-only; older G9 high-resolution develops |
| Olympus E-M1 Mark III | Normal and high-resolution ORF/ORI | Compressed sensor decoding remains preview-only |
| Pentax K-3 Mark III, K-1 | PEF, DNG, pixel-shift | Compressed PEF remains preview-only; multi-frame pixel-shift fusion is not implemented |
| Leica Q2, Ricoh GR III | DNG | File color calibration retained; no appearance certification against Photoshop |

Colors were compared with each camera's embedded JPEG, which is a useful appearance
reference but includes its picture style, local processing and lens corrections.
The fit's reported mean ΔE is measured across its proxy grid, including held-out
cells; acceptance separately requires a held-out improvement. It is not an
independent full-frame accuracy measurement or a Photoshop/ACR oracle. Examples
from the supplied files improved from 12.51 to 3.46 (X-T10), 16.99 to 4.72 and
13.04 to 5.75 (GFX 100S), and 7.84 to 3.76 (GFX 100 II). The ISO 12800 DPReview
X-T5 remained less close, 11.65 to 8.23. Noise, local contrast, lens geometry and
strong color differences remain useful follow-up work. No parity percentage or
G14 effort estimate is raised by these sample results.

The eight Z7 files previously rejected their camera references with lightness
correlation around 0.65. Bounded proxy alignment now permits the fit; their
reported proxy errors improve from 18.82–19.02 to 7.11–7.38. This is an appearance
approximation, not a substitute for measured Z7 calibration or full lens correction.

## Verification and performance

The RAW and IO suites, strict clippy, 29-crate layering and the L0–L6 WebAssembly
gate pass. The full real-file RAW/IO corpus ran during the audit; the expanded
13-file pinned RAW corpus and the IO real-file corpus were rechecked after the
final color change. The audit example's edge-block regression passes. AHD pixels
are identical across different scratch-band concurrency limits.

Isolated 24 MP release development timings (milliseconds; one before run,
median of three final runs on the same machine) show no general speed tradeoff:

| Input | Bilinear before → after | MHC before → after | AHD before → after |
|---|---:|---:|---:|
| Uncompressed DNG | 59 → 59 | 72 → 70 | 214 → 216 |
| Lossless-JPEG DNG tiles | 58 → 59 | 71 → 70 | 220 → 211 |
| CR2 lossless JPEG | 62 → 66 | 74 → 76 | 222 → 219 |

These are development-only timings, excluding file read, fit, document construction
and display. The 100 MP supplied Fujifilm developments took about 1.3–1.5 seconds
in the final full-import audit; that run used the optimized development profile
and concurrent IO testing, so it is not an isolated release benchmark. The wider
quick performance suite also ran; it still lists existing unrelated over-budget
or unmeasured scenarios, which this change does not claim to resolve.

## Repeat the audit

`cargo run --release -p photocraft-io --example camera_audit -- FILE...` emits
one JSON line per input and continues on errors. It exercises full sensor decode,
default AHD development, optional embedded-JPEG fitting and document import, or
actual JPEG fallback import. It records dimensions, depth, profile presence,
warnings, timings and fit measurements. `--png DIR` additionally writes bounded
sRGB comparison PNGs and the original embedded JPEG. Keep private files and these
outputs outside tracked source; all audit assets here stayed in ignored `plan/`.

`cargo xtask corpus --pixls` fetches the pinned Nikon regression exposures from
`xtask/src/corpus_pins.rs`, verifies `xtask/pixls.sha256`, and keeps the license
with the ignored corpus. `cargo test -p photocraft-raw --features corpus --test
raw_corpus` exercises these and the existing decode oracles. Synthetic regressions
also cover CR3 truncation, invalid tables, overflowing offsets, nested containers,
packed NEF layout, metadata crops and development memory boundaries.

The handoff includes the public sample manifest with SHA-256 values and all public
JSON results. Index descriptions are sometimes mislabeled: measured file metadata,
rather than the repository's mode string, determines bit depth and codec.

## Sources

- [DPReview sample galleries](https://www.dpreview.com/sample-galleries/) and the
  [X-T5 review gallery](https://www.dpreview.com/sample-galleries/0248558391/fujifilm-x-t5-review-sample-gallery/655603).
  The downloaded RAF is photo `2030435045`; gallery JPEG edits are not color oracles.
- [raw.pixls.us](https://raw.pixls.us/) and its
  [repository index](https://raw.pixls.us/json/getrepository.php?set=all).
- [Laurent Clévy's Canon CR3 format description](https://github.com/lclevy/canon_cr3/blob/master/readme.md),
  ISO BMFF container and camera JPEG descriptions; no third-party decoder code copied.
- [Bill Claff's NEF Compression tables](https://www.photonstophotos.net/NikonInfo/NEF_Compression.htm),
  and [ExifTool's Nikon tag documentation](https://exiftool.sourceforge.net/TagNames/Nikon.html),
  for compression modes, MakerNotes0x51 and CropArea. Sensor packing and padding
  were measured independently from the CC0 files.
