//! PCX (ZSoft Paintbrush) decode support, read-only.
//!
//! Fixed 128-byte header + per-scanline RLE, 1/4/8-bit with palette.
//! Public domain/open spec, widely supported by classic image libraries.

use crate::Format;
use crate::error::CodecError;
use crate::image::{Image, SampleType};
use crate::options::Limits;

const F: Format = Format::Pcx;

/// Error message when the `pcx` feature is not enabled.
#[cfg(not(feature = "pcx"))]
pub(crate) const NOT_IN_BUILD: &str = "PCX support isn't included in this build of PhotoCraft";

#[cfg(not(feature = "pcx"))]
pub(crate) fn decode(_bytes: &[u8], _limits: &Limits, _keep_orientation: bool) -> Result<Image, CodecError> {
    Err(CodecError::unsupported(F, NOT_IN_BUILD))
}

/// PCX header structure (128 bytes).
#[derive(Debug, Clone, Copy)]
struct PcxHeader {
    manufacturer: u8,
    version: u8,
    encoding: u8,
    bits_per_pixel: u8,
    xmin: u16,
    ymin: u16,
    xmax: u16,
    ymax: u16,
    hres: u16,
    vres: u16,
    palette_origin: u16,
    palette_size: u16,
    reserved: [u8; 54],
    color_planes: u16,
    bytes_per_line: u16,
    palette_info: u16,
    fill: u16,
}

/// Decode a PCX image using the standard RLE palette approach.

#[cfg(feature = "pcx")]
pub(crate) fn decode(bytes: &[u8], _limits: &Limits, _keep_orientation: bool) -> Result<Image, CodecError> {
    if bytes.len() < 128 {
        return Err(CodecError::malformed(F, "PCX too short"));
    }

    // Parse header
    let header = unsafe { std::slice::from_raw_parts(bytes.as_ptr() as *const u8, 128) };
    let hdr = PcxHeader {
        manufacturer: header[0],
        version: header[1],
        encoding: header[2],
        bits_per_pixel: header[3],
        xmin: u16::from_le_bytes([header[4], header[5]]),
        ymin: u16::from_le_bytes([header[6], header[7]]),
        xmax: u16::from_le_bytes([header[8], header[9]]),
        ymax: u16::from_le_bytes([header[10], header[11]]),
        hres: u16::from_le_bytes([header[12], header[13]]),
        vres: u16::from_le_bytes([header[14], header[15]]),
        palette_origin: u16::from_le_bytes([header[16], header[17]]),
        palette_size: u16::from_le_bytes([header[18], header[19]]),
        reserved: [0u8; 54],
        color_planes: u16::from_le_bytes([header[20], header[21]]),
        bytes_per_line: u16::from_le_bytes([header[22], header[23]]),
        palette_info: u16::from_le_bytes([header[24], header[25]]),
        fill: u16::from_le_bytes([header[26], header[27]]),
    };

    // Validate
    if hdr.manufacturer != 0x0A {
        return Err(CodecError::malformed(F, "PCX: invalid manufacturer byte"));
    }
    if hdr.encoding != 0x01 && hdr.encoding != 0x02 {
        return Err(CodecError::malformed(F, format!("PCX: unsupported encoding {}", hdr.encoding)));
    }
    if hdr.bits_per_pixel != 8 {
        return Err(CodecError::malformed(F, format!("PCX: {} bpp not supported", hdr.bits_per_pixel)));
    }

    // Image dimensions
    let width = (hdr.xmax - hdr.xmin + 1) as usize;
    let height = (hdr.ymax - hdr.ymin + 1) as usize;
    let bytes_per_line = hdr.bytes_per_line as usize;

    // Find palette at end of file (last 48 bytes for 256-color palette)
    let palette_start = bytes.len() - 48;
    if palette_start < 128 {
        return Err(CodecError::malformed(F, "PCX: palette too close to start"));
    }
    let palette_bytes = &bytes[palette_start..];

    // Build palette: 256 entries of [R, G, B, A=255]
    let mut palette: Vec<(u8, u8, u8, u8)> = Vec::with_capacity(256);
    for i in 0..256 {
        let idx = i * 3;
        if idx + 2 < palette_bytes.len() {
            let r = palette_bytes[idx];
            let g = palette_bytes[idx + 1];
            let b = palette_bytes[idx + 2];
            palette.push((r, g, b, 255));
        } else {
            palette.push((0, 0, 0, 255)); // default
        }
    }

    // Decode RLE scanlines
    // PCX RLE scheme:
    // - If byte >= 0xC0: run-length encoded pair (count = byte - 0xC0 + 1, value = next byte)
    // - If byte < 0xC0: unencoded byte (single pixel)
    let mut img_data = vec![0u8; width * height];
    let mut row_idx = 0;
    let payload_start = 128usize;
    let scanline_end = payload_start + height * bytes_per_line;
    let scanline_data = &bytes[payload_start..scanline_end.min(bytes.len() - 48)];

    while row_idx < height {
        let line_start = payload_start + row_idx * bytes_per_line;
        let line_end = line_start + bytes_per_line;
        let scanline = &bytes[line_start..line_end.min(bytes.len() - 48)];

        let mut col_idx = 0;
        let mut s_idx = 0;

        while col_idx < width && s_idx < scanline.len() {
            let ctrl = scanline[s_idx];
            s_idx += 1;

            if ctrl >= 0xC0 {
                // Run-length encoded pair
                let run_count = (ctrl - 0xC0) as usize + 1;
                let run_value = if s_idx < scanline.len() {
                    scanline[s_idx]
                } else {
                    break;
                };
                s_idx += 1;

                let remaining = width - col_idx;
                let repeat = run_count.min(remaining);
                for _ in 0..repeat {
                    img_data[row_idx * width + col_idx] = run_value;
                    col_idx += 1;
                }
            } else {
                // Unencoded byte
                if col_idx >= width { break; }
                img_data[row_idx * width + col_idx] = ctrl;
                col_idx += 1;
            }
        }

        row_idx += 1;
        if row_idx >= height { break; }
    }

    // Convert palette indices to RGB
    let mut rgb_data = vec![0u8; width * height * 3];
    for (i, &palette_idx) in img_data.iter().enumerate() {
        if (*palette_idx as usize) < palette.len() {
            let (r, g, b, _) = palette[palette_idx as usize];
            rgb_data[i * 3] = r;
            rgb_data[i * 3 + 1] = g;
            rgb_data[i * 3 + 2] = b;
        }
    }

    // Build Image
    let metadata = ();
    Ok(Image::new_rgb(width, height, rgb_data, metadata))
}

