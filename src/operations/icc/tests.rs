use crate::test_utils::*;
use super::*;

#[test]
fn apply_icc_profile() {
    let mut image = create_test_image_u8(100, 100, ColorSpace::RGB);
    image.metadata_mut().set_icc_chunk(ColorProfile::new_srgb().encode().unwrap());
    let target = std::fs::read("tests/files/icc/tinysrgb.icc").unwrap();
    ApplyICC::new(&target).unwrap().execute_impl(&mut image).unwrap();
    assert_eq!(image.metadata().icc_chunk().unwrap(), &target);
}

#[test]
fn skip_icc_profile() {
    let mut image = create_test_image_u8(100, 100, ColorSpace::RGB);
    ApplySRGB.execute_impl(&mut image).unwrap();
    assert_eq!(image.metadata().icc_chunk(), None);
}

#[test]
fn apply_srgb_profile() {
    let mut image = create_test_image_u8(100, 100, ColorSpace::RGB);
    image.metadata_mut().set_icc_chunk(std::fs::read("tests/files/icc/tinysrgb.icc").unwrap());
    ApplySRGB.execute_impl(&mut image).unwrap();
    assert_eq!(image.metadata().icc_chunk().unwrap(), &ColorProfile::new_srgb().encode().unwrap());
}

#[test]
fn apply_icc_profile_to_u16_without_panicking() {
    let mut image = create_test_image_u16(8, 8, ColorSpace::RGB);
    let profile = ColorProfile::new_srgb().encode().unwrap();
    image.metadata_mut().set_icc_chunk(profile.clone());
    ApplyICC::new(&profile).unwrap().execute(&mut image).unwrap();
    assert_eq!(image.depth().bit_type(), BitType::U16);
    assert_eq!(image.dimensions(), (8, 8));
}
