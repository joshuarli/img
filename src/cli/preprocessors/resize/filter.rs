use anyhow::anyhow;
use fast_image_resize as fr;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResizeFilter {
    Nearest,
    Box,
    Bilinear,
    Hamming,
    CatmullRom,
    Mitchell,
    Lanczos3,
}

impl std::fmt::Display for ResizeFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Nearest => "nearest", Self::Box => "box", Self::Bilinear => "bilinear",
            Self::Hamming => "hamming", Self::CatmullRom => "catmull-rom",
            Self::Mitchell => "mitchell", Self::Lanczos3 => "lanczos3",
        })
    }
}

impl std::str::FromStr for ResizeFilter {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "nearest" => Ok(Self::Nearest), "box" => Ok(Self::Box), "bilinear" => Ok(Self::Bilinear),
            "hamming" => Ok(Self::Hamming), "catmull-rom" => Ok(Self::CatmullRom),
            "mitchell" => Ok(Self::Mitchell), "lanczos3" => Ok(Self::Lanczos3),
            _ => Err(anyhow!("invalid variant: {s}")),
        }
    }
}

impl From<ResizeFilter> for fr::ResizeAlg {
    fn from(value: ResizeFilter) -> Self {
        match value {
            ResizeFilter::Nearest => fr::ResizeAlg::Nearest,
            ResizeFilter::Box => fr::ResizeAlg::Convolution(fr::FilterType::Box),
            ResizeFilter::Bilinear => fr::ResizeAlg::Convolution(fr::FilterType::Bilinear),
            ResizeFilter::Hamming => fr::ResizeAlg::Convolution(fr::FilterType::Hamming),
            ResizeFilter::CatmullRom => fr::ResizeAlg::Convolution(fr::FilterType::CatmullRom),
            ResizeFilter::Mitchell => fr::ResizeAlg::Convolution(fr::FilterType::Mitchell),
            ResizeFilter::Lanczos3 => fr::ResizeAlg::Convolution(fr::FilterType::Lanczos3),
        }
    }
}
