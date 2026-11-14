use std::{env,fs,path::Path};
use img::still::StillImage;
fn norm(img:&StillImage)->(u32,u32,Vec<u8>) {
    let (w,h)=img.dimensions();
    let s=img.samples();
    let channels=s.len()/(w as usize*h as usize);
    match channels {
        1=>(w,h,(0..s.len()).flat_map(|i|[s[i];3]).collect()),
        2=>(w,h,(0..s.len()/2).flat_map(|i|[s[i*2];3]).collect()),
        3=>(w,h,s),
        4=>(w,h,(0..s.len()/4).flat_map(|i|[s[i*4],s[i*4+1],s[i*4+2]]).collect()),
        _=>panic!("bad channels"),
    }
}
fn main() {
    let args:Vec<String>=env::args().collect();
    let a=StillImage::decode(&fs::read(Path::new(&args[1])).unwrap()).unwrap();
    let b=StillImage::decode(&fs::read(Path::new(&args[2])).unwrap()).unwrap();
    let (aw,ah,ra)=norm(&a);let (bw,bh,rb)=norm(&b);
    assert_eq!((aw,ah),(bw,bh));
    assert_eq!(ra.len(),rb.len());
    let mut max=0u16;let mut sum=0u64;let mut ch_sum=[0u64;3];let mut hist=[0u64;6];let mut ch_max=[0u16;3];let mut n=0u64;
    for i in 0..ra.len() {
        let d=(i32::from(ra[i])-i32::from(rb[i])).abs() as u16;
        max=max.max(d);sum+=u64::from(d);ch_sum[i%3]+=d as u64;ch_max[i%3]=ch_max[i%3].max(d);
        hist[match d {0=>0,1=>1,2..=4=>2,5..=8=>3,9..=16=>4,_=>5}]+=1;n+=1;
    }
    println!("max={max} mean={:.4} mean_ch={:.4?} ch_max={ch_max:?} hist(0,1,2-4,5-8,9-16,>16)={hist:?}",sum as f64/n as f64, ch_sum.map(|s| s as f64));
}
