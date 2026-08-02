//! 生成 demo 用测试图:cargo run -p qingui-codegen --example make_demo_assets
use image::codecs::gif::GifEncoder;
use image::{Delay, Frame, Rgba, RgbaImage};

fn main() {
    let dir = std::path::Path::new("qingui/examples/assets");
    std::fs::create_dir_all(dir).unwrap();
    // logo.png:48x24 蓝底白斜纹
    let mut img = RgbaImage::from_pixel(48, 24, Rgba([40, 80, 200, 255]));
    for x in 0..48 {
        for y in 0..24 {
            if (x + y) % 8 < 2 {
                img.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
    }
    img.save(dir.join("logo.png")).unwrap();
    // anim.gif:16x16,3 帧纯色(红/绿/蓝),各 300ms
    let frames: Vec<Frame> = [[255u8, 0, 0], [0, 255, 0], [0, 0, 255]]
        .into_iter()
        .map(|c| Frame::from_parts(RgbaImage::from_pixel(16, 16, Rgba([c[0], c[1], c[2], 255])),
                                   0, 0, Delay::from_numer_denom_ms(300, 1)))
        .collect();
    let mut enc = GifEncoder::new(std::fs::File::create(dir.join("anim.gif")).unwrap());
    enc.encode_frames(frames.into_iter()).unwrap();
}
