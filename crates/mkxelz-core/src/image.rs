use crate::error::Result;
use std::path::Path;

/// Plain RGBA8 image (straight alpha).
#[derive(Clone, Debug)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Image {
    pub fn new(width: u32, height: u32) -> Self {
        Image {
            width,
            height,
            rgba: vec![0; (width * height * 4) as usize],
        }
    }

    /// Copy a region of `src` into `self`. With `over`, alpha-blends instead of copying.
    #[allow(clippy::too_many_arguments)]
    pub fn blit_region(
        &mut self,
        src: &Image,
        sx: u32,
        sy: u32,
        w: u32,
        h: u32,
        dx: u32,
        dy: u32,
        over: bool,
    ) {
        for y in 0..h {
            for x in 0..w {
                let (spx, spy, dpx, dpy) = (sx + x, sy + y, dx + x, dy + y);
                if spx >= src.width || spy >= src.height || dpx >= self.width || dpy >= self.height {
                    continue;
                }
                let si = ((spy * src.width + spx) * 4) as usize;
                let di = ((dpy * self.width + dpx) * 4) as usize;
                let sa = src.rgba[si + 3] as u32;
                if !over || sa == 255 {
                    for c in 0..4 {
                        self.rgba[di + c] = src.rgba[si + c];
                    }
                } else if sa > 0 {
                    let da = self.rgba[di + 3] as u32;
                    let out_a = sa + da * (255 - sa) / 255;
                    for c in 0..3 {
                        let s = src.rgba[si + c] as u32;
                        let d = self.rgba[di + c] as u32;
                        let v = (s * sa + d * da * (255 - sa) / 255) / out_a.max(1);
                        self.rgba[di + c] = v.min(255) as u8;
                    }
                    self.rgba[di + 3] = out_a.min(255) as u8;
                }
            }
        }
    }

    pub fn encode_png(&self) -> Result<Vec<u8>> {
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut buf, self.width, self.height);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut writer = enc.write_header()?;
            writer.write_image_data(&self.rgba)?;
            writer.finish()?;
        }
        Ok(buf)
    }

    pub fn save_png(&self, path: &Path) -> Result<()> {
        let bytes = self.encode_png()?;
        std::fs::write(path, bytes)?;
        Ok(())
    }
}
