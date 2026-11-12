use dav1d::{Decoder,Error,PixelLayout,PlanarImageComponent,pixel::YUVRange as NativeRange};
pub fn decode(bytes:&[u8])->Vec<u8> {
    let parser=zenavif_parse::AvifParser::from_bytes(bytes).unwrap();let data=parser.primary_data().unwrap();
    let mut settings=dav1d::Settings::new();settings.set_max_frame_delay(1);
    let mut decoder=Decoder::with_settings(&settings).unwrap();
    assert!(matches!(decoder.send_data(data.to_vec(),None,None,None),Ok(())|Err(Error::Again)));
    let picture=match decoder.get_picture() {Ok(p)=>p,Err(Error::Again)=>{decoder.send_pending_data().unwrap();decoder.get_picture().unwrap()},Err(e)=>panic!("native AV1 decode: {e}")};
    assert_eq!((picture.width(),picture.height()),(48,32));assert_eq!(picture.bit_depth(),8);
    let image=yuvutils_rs::YuvPlanarImage {y_plane:&picture.plane(PlanarImageComponent::Y),y_stride:picture.stride(PlanarImageComponent::Y),u_plane:&picture.plane(PlanarImageComponent::U),u_stride:picture.stride(PlanarImageComponent::U),v_plane:&picture.plane(PlanarImageComponent::V),v_stride:picture.stride(PlanarImageComponent::V),width:picture.width(),height:picture.height()};
    let range=if picture.color_range()==NativeRange::Full {yuvutils_rs::YuvRange::Full} else {yuvutils_rs::YuvRange::Limited};
    let mut rgba=vec![0;48*32*4];let matrix=yuvutils_rs::YuvStandardMatrix::Bt601;
    match picture.pixel_layout() {
        PixelLayout::I420=>yuvutils_rs::yuv420_to_rgba(&image,&mut rgba,48*4,range,matrix),
        PixelLayout::I422=>yuvutils_rs::yuv422_to_rgba(&image,&mut rgba,48*4,range,matrix),
        PixelLayout::I444=>yuvutils_rs::yuv444_to_rgba(&image,&mut rgba,48*4,range,matrix),
        _=>panic!("unexpected monochrome fixture"),
    }.unwrap();
    rgba.chunks_exact(4).flat_map(|p|p[..3].iter().copied()).collect()
}
