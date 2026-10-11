//! Repeatable real-camera audit: one JSON result per file, continuing on errors.
//! `cargo run --release -p photocraft-io --example camera_audit -- [--png DIR] FILE...`
//! Saves bounded sRGB previews; full-resolution develops run serially, so even
//! 100MP cameras do not require several full float RGB copies for comparison.

use photocraft_cms::{Builtin, Intent, Transform};
use photocraft_codecs::{ChannelLayout, EncodeOptions, Format, Image};
use photocraft_raw::{DevelopOptions, Developed};
use serde_json::{Value, json};
use std::{
    error::Error,
    path::{Path, PathBuf},
    time::Instant,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Average in linear ProPhoto before the colour conversion, with at most 1200
/// output pixels on the long edge. Pixel-level colour fitting remains separate.
fn preview(d: &Developed) -> Result<Image> {
    let (w, h) = (d.width as usize, d.height as usize);
    let step = w.max(h).div_ceil(1200).max(1);
    let (ow, oh) = (w.div_ceil(step), h.div_ceil(step));
    let mut small = Vec::with_capacity(ow * oh * 3);
    for oy in 0..oh {
        for ox in 0..ow {
            let (x0, y0) = (ox * step, oy * step);
            let (x1, y1) = ((x0 + step).min(w), (y0 + step).min(h));
            let mut sum = [0.0f64; 3];
            for y in y0..y1 {
                for x in x0..x1 {
                    let at = (y * w + x) * 3;
                    let pixel = d.rgb.get(at..at + 3).ok_or("short developed RGB buffer")?;
                    for (total, v) in sum.iter_mut().zip(pixel) {
                        *total += (f64::from(*v) / 65535.0).powf(1.8);
                    }
                }
            }
            let count = ((x1 - x0) * (y1 - y0)) as f64;
            small.extend(sum.map(|v| ((v / count).powf(1.0 / 1.8) * 65535.0 + 0.5) as u16));
        }
    }
    let mut rgb = vec![0u16; small.len()];
    let t = Transform::new(Builtin::ProPhotoCompat.profile(), Builtin::Srgb.profile(), Intent::RelativeColorimetric, false)?;
    t.convert_u16(&small, 3, &mut rgb, 3, false);
    Ok(Image::from_u16(ow as u32, oh as u32, ChannelLayout::Rgb, &rgb)?)
}

fn save(dir: Option<&Path>, stem: &str, suffix: &str, image: &Image) -> Result<()> {
    if let Some(dir) = dir {
        std::fs::write(dir.join(format!("{stem}-{suffix}.png")), photocraft_codecs::encode(image, Format::Png, &EncodeOptions::default())?)?;
    }
    Ok(())
}

fn audit(path: &Path, dir: Option<&Path>) -> Result<Value> {
    let bytes = std::fs::read(path)?;
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("raw");
    let stem = path.file_stem().and_then(|n| n.to_str()).unwrap_or("raw");
    let opts = DevelopOptions::default();
    let started = Instant::now();
    let sensor = match photocraft_raw::decode(&bytes, &opts.limits) {
        Ok(s) => s,
        Err(e) => {
            let r = photocraft_io::raw::import_raw(name, &bytes)?;
            if let (Some(dir), Some(p)) = (dir, photocraft_raw::embedded_preview(&bytes)) {
                std::fs::write(dir.join(format!("{stem}-camera.jpg")), p.jpeg)?;
            }
            return Ok(json!({"file":name,"result":"camera JPEG fallback","decode_error":e.to_string(),
                "dimensions":[r.document.size.width,r.document.size.height],"depth":format!("{:?}",r.document.depth),"warnings":r.warnings}));
        }
    };
    let decode_ms = started.elapsed().as_secs_f64() * 1000.0;
    let started = Instant::now();
    let fit = if sensor.color.calibrations.is_empty() {
        photocraft_io::raw::fit_preview_look(&bytes, &sensor, &opts)
    } else {
        Err("file colour calibration retained".into())
    };
    let fit_ms = started.elapsed().as_secs_f64() * 1000.0;
    let fitting = match &fit {
        Ok(l) => json!({"used":true,"delta_e_fallback":l.delta_e_fallback,"delta_e":l.delta_e}),
        Err(e) => json!({"used":false,"reason":e}),
    };
    if dir.is_some() {
        let before = photocraft_raw::develop_sensor(&sensor, &opts)?;
        save(dir, stem, "before", &preview(&before)?)?;
    }
    let options = DevelopOptions { camera_look: fit.ok(), ..opts };
    let started = Instant::now();
    let after = photocraft_raw::develop_sensor(&sensor, &options)?;
    let develop_ms = started.elapsed().as_secs_f64() * 1000.0;
    if dir.is_some() {
        save(dir, stem, "after", &preview(&after)?)?;
    }
    if let (Some(dir), Some(p)) = (dir, photocraft_raw::embedded_preview(&bytes)) {
        std::fs::write(dir.join(format!("{stem}-camera.jpg")), p.jpeg)?;
    }
    let c = sensor.crop;
    let mut result = json!({"file":name,"result":"sensor developed","make":sensor.make,"model":sensor.model,
        "sensor":[sensor.width,sensor.height],"crop":[c.x,c.y,c.width,c.height],"dimensions":[after.width,after.height],
        "depth":"U16","black":sensor.black.values,"white":sensor.white,"white_balance":after.info.wb_multipliers,
        "orientation":sensor.orientation,"fit":fitting,"decode_ms":decode_ms,"fit_ms":fit_ms,"develop_ms":develop_ms,"warnings":after.warnings});
    // Check the application import too, releasing the full development buffers
    // first so the audit does not retain two 100MP images at once.
    drop(after);
    drop(sensor);
    let imported = photocraft_io::raw::import_raw(name, &bytes)?;
    result["import"] = json!({"dimensions":[imported.document.size.width,imported.document.size.height],
        "depth":format!("{:?}",imported.document.depth),"icc_bytes":imported.document.icc_profile.as_ref().map(|p| p.len())});
    Ok(result)
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut dir: Option<PathBuf> = None;
    let mut files = Vec::new();
    while let Some(arg) = args.next() {
        if arg == "--png" {
            dir = Some(PathBuf::from(args.next().ok_or("--png needs a directory")?));
        } else {
            files.push(PathBuf::from(arg));
        }
    }
    if let Some(dir) = &dir {
        std::fs::create_dir_all(dir)?;
    }
    for path in files {
        let result = match audit(&path, dir.as_deref()) {
            Ok(result) => result,
            Err(e) => json!({"file":path.file_name().and_then(|n| n.to_str()),"result":"error","error":e.to_string()}),
        };
        println!("{}", serde_json::to_string(&result)?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_preview_includes_partial_edge_blocks() {
        let bytes = photocraft_raw::testgen::DngSpec::cfa(1301, 9, vec![12000; 1301 * 9]).build();
        let d = photocraft_raw::develop(&bytes, &DevelopOptions::default()).unwrap();
        let p = preview(&d).unwrap();
        assert_eq!(p.dimensions(), (651, 5));
        assert_eq!(p.data().len(), 651 * 5 * 3 * 2);
    }
}
