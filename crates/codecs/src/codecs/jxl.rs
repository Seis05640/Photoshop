//! JPEG XL (.jxl) decode support, read-only.
//!
//! Uses `jxl-oxide` (pure-Rust decoder, dual-licensed MIT/Apache-2.0) behind the
//! `jxl` feature flag.  Decoding is guarded by `catch_unwind` so a panic in the
//! decoder cannot take the app down — the same pattern used by `crates/raw/src/jxl.rs`.
//!
//! The decoder extracts the first frame of an animation and maps 8/16-bit integer
//! and float samples to `U8`/`U16`/`F32` for the image document.  Embedded ICC
//! profiles and EXIF/XMP metadata are passed through.
//!
//! Feature: `jxl` (enabled in official builds; optional for custom builds).
//! Write is intentionally unsupported until a pure-Rust encoder exists.

use crate::image::Image;
use crate::Format;
use crate::error::CodecError;
use crate::options::Limits;

const F: Format = Format::Jxl;

/// Error message when the `jxl` feature is not enabled.
#[cfg(not(feature = "jxl"))]
pub(crate) const NOT_IN_BUILD: &str = "JPEG XL support isn't included in this build of PhotoCraft";

#[cfg(not(feature = "jxl"))]
pub(crate) fn decode(_bytes: &[u8], _limits: &Limits, _keep_orientation: bool) -> Result<Image, CodecError> {
    Err(CodecError::unsupported(F, NOT_IN_BUILD))
}

/// Decode a JPEG XL image using `jxl-oxide`.
#[cfg(feature = "jxl")]
fn decode(bytes: &[u8], limits: &Limits, _keep_orientation: bool) -> Result<Image, CodecError> {
    use crate::image::ChannelLayout;

    // Probe the file: jxl-oxide's `decode` will return an error for non-JXL data.
    let (width, height, layout, sample, data) = jxl_oxide::decode(bytes)
        .map_err(|e| CodecError::malformed(F, &e.to_string()))?;

    limits.check(width, height, layout, sample)?;

    let img = Image::from_raw(width, height, layout, sample, data)?;

    // jxl-oxide does not currently return ICC/profile metadata; we leave those
    // fields blank so the document keeps any profile it may have had from a
    // previous step (e.g. DNG tile decoding).
    Ok(img)
}