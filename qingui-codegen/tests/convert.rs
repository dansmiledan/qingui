use image::codecs::gif::GifEncoder;
use image::{Delay, Frame, Rgba, RgbaImage};
use std::fs;

/// 现场生成 2x2 png(左上角纯红,其余纯绿)与 2 帧 gif(帧1 全红 80ms,帧2 全蓝 120ms)
fn make_assets(dir: &std::path::Path) {
    let mut png = RgbaImage::from_pixel(2, 2, Rgba([0, 255, 0, 255]));
    png.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
    png.save(dir.join("logo.png")).unwrap();

    let f1 = Frame::from_parts(RgbaImage::from_pixel(2, 2, Rgba([255, 0, 0, 255])),
                               0, 0, Delay::from_numer_denom_ms(80, 1));
    let f2 = Frame::from_parts(RgbaImage::from_pixel(2, 2, Rgba([0, 0, 255, 255])),
                               0, 0, Delay::from_numer_denom_ms(120, 1));
    let mut enc = GifEncoder::new(fs::File::create(dir.join("anim.gif")).unwrap());
    enc.encode_frames(vec![f1, f2].into_iter()).unwrap();
}

#[test]
fn convert_generates_expected_images_rs() {
    let tmp = std::env::temp_dir().join(format!("qg-codegen-{}", std::process::id()));
    let assets = tmp.join("assets");
    let out = tmp.join("out");
    fs::create_dir_all(&assets).unwrap();
    fs::create_dir_all(&out).unwrap();
    make_assets(&assets);

    qingui_codegen::convert(assets.to_str().unwrap(), out.to_str().unwrap()).unwrap();
    let gen = fs::read_to_string(out.join("images.rs")).unwrap();

    // 静态图:单帧、2x2、8 字节、delay 0
    assert!(gen.contains("pub static LOGO: qingui::widgets::image::ImageData"));
    assert!(gen.contains("delays_ms: &[0]"));
    // gif:两帧、延时 80/120
    assert!(gen.contains("pub static ANIM: qingui::widgets::image::ImageData"));
    assert!(gen.contains("delays_ms: &[80, 120]"));
    // 帧像素:png 的 (0,0) 纯红 → 0xF800 小端 = [0x00, 0xF8] 在最前
    assert!(gen.contains("0x00, 0xF8"));

    fs::remove_dir_all(&tmp).ok();
}
