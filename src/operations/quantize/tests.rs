use crate::test_utils::*;
use zune_image::image::Image;

use super::*;

#[test]
fn quantize_u8() {
    let quantize = Quantize::new(75, None);
    let mut image = create_test_image_u8(200, 200, ColorSpace::RGBA);

    let result = quantize.execute(&mut image);

    dbg!(&result);

    assert!(result.is_ok());
}

#[test]
fn quantize_refuses_varying_alpha() {
    let quantize = Quantize::new(75, None);
    let mut image = Image::from_fn(4, 4, ColorSpace::RGBA, |x, y, px: &mut [u8; 4]| {
        px[0] = x as u8; px[1] = y as u8; px[2] = 55; px[3] = if x == 0 { 128 } else { 255 };
    });

    let result = quantize.execute(&mut image);

    assert!(result.is_err());
}

#[test]
fn dither_u8() {
    let quantize = Quantize::new(75, Some(0.75));
    let mut image = create_test_image_u8(200, 200, ColorSpace::RGBA);

    let result = quantize.execute(&mut image);

    dbg!(&result);

    assert!(result.is_ok());
}
