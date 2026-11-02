use std::path::{Path, PathBuf};
use std::process::Command;

use fs_core::{FsError, FsResult};

/// Converts `path` into `target` (an extension such as "png" or "mp3") next to the original, then moves
/// the original to the Trash. Returns the new file's path.
pub fn convert_file(path: &Path, target: &str) -> FsResult<PathBuf> {
    let dir = path.parent().ok_or_else(|| FsError::InvalidTarget("No parent folder".into()))?;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let (stem, _) = fs_core::names::split_ext(&name);
    let tmp = dir.join(format!(".{stem}.rfconvert.{target}"));
    let scratch = tempfile_dir()?;

    let result = match target {
        "png" | "jpeg" | "heic" | "tiff" => sips(path, target, &tmp),
        "webp" => webp(path, &scratch, &tmp),
        "mp4" | "mov" => avconvert(path, "PresetHighestQuality", &tmp),
        "m4a" => avconvert(path, "PresetAppleM4A", &tmp),
        "wav" => afconvert(path, "LEI16", &tmp),
        "mp3" => mp3(path, &scratch, &tmp),
        _ => Err(format!("Can't convert to .{target}")),
    };
    let _ = std::fs::remove_dir_all(&scratch);
    if let Err(msg) = result {
        let _ = std::fs::remove_file(&tmp);
        return Err(FsError::Io(format!("Couldn't convert “{name}”: {msg}")));
    }

    let dest = fs_core::names::unique_name(dir, &format!("{stem}.{target}"), |p| p.exists());
    std::fs::rename(&tmp, &dest).map_err(|e| FsError::from_io(e, "Couldn't save the converted file"))?;
    fs_core::ops::move_to_trash(path)
        .map_err(|e| FsError::Io(format!("Converted, but couldn't move “{name}” to the Trash: {e}")))?;
    Ok(dest)
}

fn tempfile_dir() -> FsResult<PathBuf> {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
    let dir = std::env::temp_dir().join(format!("findr-convert-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| FsError::from_io(e, "Couldn't create a temporary folder"))?;
    Ok(dir)
}

fn run(cmd: &mut Command) -> Result<(), String> {
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() {
        return Ok(());
    }
    let (err, stdout) = (String::from_utf8_lossy(&out.stderr), String::from_utf8_lossy(&out.stdout));
    let msg = err.lines().chain(stdout.lines()).map(str::trim).find(|l| !l.is_empty());
    Err(msg.unwrap_or("the converter failed").to_string())
}

fn sips(src: &Path, format: &str, out: &Path) -> Result<(), String> {
    let mut cmd = Command::new("/usr/bin/sips");
    cmd.args(["-s", "format", format]);
    if matches!(format, "jpeg" | "heic") {
        cmd.args(["-s", "formatOptions", "high"]);
    }
    run(cmd.arg(src).arg("--out").arg(out))
}

fn webp(src: &Path, scratch: &Path, out: &Path) -> Result<(), String> {
    let png = scratch.join("frame.png");
    sips(src, "png", &png)?;
    let image = (|| {
        use image::ImageDecoder;
        let mut decoder = image::ImageReader::open(&png)?.into_decoder()?;
        // sips keeps the camera's EXIF rotation as metadata; WebP has no reliable equivalent, so bake it in.
        let orientation = decoder.orientation()?;
        let mut image = image::DynamicImage::from_decoder(decoder)?;
        image.apply_orientation(orientation);
        Ok::<_, image::ImageError>(image.to_rgba8())
    })()
    .map_err(|e| e.to_string())?;
    let data = webp::Encoder::from_rgba(&image, image.width(), image.height()).encode(85.0);
    std::fs::write(out, &*data).map_err(|e| e.to_string())
}

fn avconvert(src: &Path, preset: &str, out: &Path) -> Result<(), String> {
    run(Command::new("/usr/bin/avconvert").args(["--preset", preset, "--source"]).arg(src).arg("--output").arg(out))
}

fn afconvert(src: &Path, data_format: &str, out: &Path) -> Result<(), String> {
    run(Command::new("/usr/bin/afconvert").args(["-f", "WAVE", "-d", data_format]).arg(src).arg(out))
}

// macOS ships no MP3 encoder, so decode to PCM with afconvert and encode with the bundled LAME.
fn mp3(src: &Path, scratch: &Path, out: &Path) -> Result<(), String> {
    use mp3lame_encoder::{Builder, FlushNoGap, InterleavedPcm, MonoPcm};

    let wav = scratch.join("audio.wav");
    afconvert(src, "LEI16@44100", &wav).map_err(|_| "it has no readable audio".to_string())?;
    let mut reader = hound::WavReader::open(&wav).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    if spec.channels > 2 {
        return Err("only mono and stereo audio can be converted to MP3".into());
    }
    let samples: Vec<i16> = reader.samples::<i16>().collect::<Result<_, _>>().map_err(|e| e.to_string())?;

    let mut builder = Builder::new().ok_or("couldn't start the MP3 encoder")?;
    let setup = (|| {
        builder.set_num_channels(spec.channels as u8)?;
        builder.set_sample_rate(spec.sample_rate)?;
        builder.set_brate(mp3lame_encoder::Bitrate::Kbps192)?;
        builder.set_quality(mp3lame_encoder::Quality::Best)
    })();
    setup.map_err(|e| e.to_string())?;
    let mut encoder = builder.build().map_err(|e| e.to_string())?;

    let mut mp3 = Vec::with_capacity(samples.len() * 5 / 4 + 7200);
    let encoded = if spec.channels == 1 {
        encoder.encode_to_vec(MonoPcm(&samples), &mut mp3)
    } else {
        encoder.encode_to_vec(InterleavedPcm(&samples), &mut mp3)
    };
    encoded.map_err(|e| e.to_string())?;
    mp3.reserve(7200);
    encoder.flush_to_vec::<FlushNoGap>(&mut mp3).map_err(|e| e.to_string())?;
    std::fs::write(out, mp3).map_err(|e| e.to_string())
}
