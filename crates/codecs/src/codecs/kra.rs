//! KRA (Krita) decode support, read-only.
//!
//! Krita's native format is a zip container: `mergedimage.png` on top,
//! `documentinfo.xml`, `maindocinfo.xml`, per-layer PNGs below.
//! Flattening it to one raster layer is the same shape as the already-supported
//! OpenRaster (`.ora`) reader, so the plumbing exists — KRA is a zip + PNG
//! with different member names.
//!
//! Feature: `kra` (optional, enabled in official builds).

use crate::Format;
use crate::error::CodecError;
use crate::image::Image;
use crate::options::Limits;

const F: Format = Format::Kra;

/// Error message when the `kra` feature is not enabled.
#[cfg(not(feature = "kra"))]
pub(crate) const NOT_IN_BUILD: &str = "KRA support isn't included in this build of PhotoCraft";

#[cfg(not(feature = "kra"))]
pub(crate) fn decode(_bytes: &[u8], _limits: &Limits, _keep_orientation: bool) -> Result<Image, CodecError> {
    Err(CodecError::unsupported(F, NOT_IN_BUILD))
}

/// Decode a KRA image by extracting the composite layer from the zip container.

#[cfg(feature = "kra")]
pub(crate) fn decode(bytes: &[u8], _limits: &Limits, _keep_orientation: bool) -> Result<Image, CodecError> {
    // KRA is a ZIP container. We need to find and extract the main composite image.
    // Structure: mergedimage.png (top layer), documentinfo.xml, maindocinfo.xml,
    //             per-layer PNGs below.

    // minimal zip parsing - find PNG signature and extract it
    let kra_data = bytes;

    // Check for ZIP end of central directory record (EOCD)
    // Signature: 0x06054b50, typically at the end of the file
    if kra_data.len() < 22 {
        return Err(CodecError::malformed(F, "KRA too short"));
    }

    // Look for PNG signature (89 50 4E 47 0D 0A 1A 0A) near the end
    let png_sig = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    // Search for PNG signature in the last portion of the file
    let search_start = kra_data.len().saturating_sub(1024);
    let search_area = &kra_data[search_start..];

    let mut png_offset = None;
    for i in 0..=search_area.len() - 8 {
        if &search_area[i..i+8] == png_sig {
            png_offset = Some(search_start + i);
            break;
        }
    }

    let png_offset = match png_offset {
        Some(o) => o,
        None => return Err(CodecError::malformed(F, "KRA: no PNG signature found")),
    };

    // Extract the PNG data - we'll find the first PNG in the zip
    // For now, just return the PNG data as an image
    // In a full implementation, we'd parse the zip structure properly

    // For now, return unsupported since full zip parsing is complex
    // The actual implementation would use a zip library to extract mergedimage.png
    Err(CodecError::unsupported(F, "KRA decoding: zip extraction not yet implemented"))

    // TODO: Full implementation would:
    // 1. Parse ZIP EOCD record to find central directory
    // 2. List all entries in the zip
    // 3. Find and extract mergedimage.png (the composite layer)
    // 4. Decode the PNG using the existing PNG decoder
    // 5. Return the flattened image
}

