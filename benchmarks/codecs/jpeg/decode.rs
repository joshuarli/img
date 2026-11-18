use std::{env, error::Error, fs, hint::black_box};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().collect();
    let data = fs::read(&args[2])?;
    match args[1].as_str() {
        "zune" => {
            let pixels = zune_jpeg::JpegDecoder::new(std::io::Cursor::new(&data)).decode()?;
            black_box(pixels);
        }
        "zen" => {
            let image = zenjpeg::decoder::Decoder::new().decode(&data, enough::Unstoppable)?;
            black_box(image);
        }
        _ => return Err("unknown decoder".into()),
    }
    Ok(())
}
