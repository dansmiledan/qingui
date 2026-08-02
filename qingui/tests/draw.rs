use qingui::draw::DrawBuf;
use qingui::{Color, Rect};

fn buf(w: i32, h: i32) -> (Vec<Color>, Rect) {
    (vec![Color::BLACK; (w * h) as usize], Rect::new(0, 0, w, h))
}

#[test]
fn fill_rect_basic() {
    let (mut px, area) = buf(10, 10);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 10 };
    d.fill_rect(Rect::new(2, 2, 3, 3), Color::RED, 255, area);
    let at = |px: &[Color], x: i32, y: i32| px[(y * 10 + x) as usize];
    assert_eq!(at(d.pixels, 2, 2), Color::RED);
    assert_eq!(at(d.pixels, 4, 4), Color::RED);
    assert_eq!(at(d.pixels, 1, 2), Color::BLACK);
    assert_eq!(at(d.pixels, 5, 5), Color::BLACK);
}

#[test]
fn fill_rect_clipped() {
    let (mut px, area) = buf(10, 10);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 10 };
    let clip = Rect::new(0, 0, 3, 10);
    d.fill_rect(Rect::new(0, 0, 10, 10), Color::RED, 255, clip);
    assert_eq!(d.pixels[(5 * 10 + 2) as usize], Color::RED); // clip 内
    assert_eq!(d.pixels[(5 * 10 + 3) as usize], Color::BLACK); // clip 外
}

#[test]
fn fill_rect_opa_blends() {
    let (mut px, area) = buf(4, 4);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 4 };
    d.clear(Color::BLACK);
    d.fill_rect(Rect::new(0, 0, 4, 4), Color::WHITE, 128, area);
    assert_eq!(d.pixels[0], Color::rgb(128, 128, 128));
}

#[test]
fn fill_rounded_corners_cut() {
    let (mut px, area) = buf(20, 20);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 20 };
    d.fill_rounded(Rect::new(0, 0, 20, 20), 6, Color::RED, 255, area);
    let at = |x: usize, y: usize| d.pixels[y * 20 + x];
    assert_eq!(at(0, 0), Color::BLACK); // 角被切掉
    assert_eq!(at(10, 10), Color::RED); // 中心保留
    assert_eq!(at(10, 0), Color::RED); // 顶边中部保留
}

#[test]
fn draw_border_ring() {
    let (mut px, area) = buf(20, 20);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 20 };
    d.draw_border(Rect::new(0, 0, 20, 20), 2, 0, Color::GREEN, 255, area);
    let at = |x: usize, y: usize| d.pixels[y * 20 + x];
    assert_eq!(at(10, 0), Color::GREEN); // 顶边
    assert_eq!(at(10, 1), Color::GREEN); // 宽度 2
    assert_eq!(at(10, 2), Color::BLACK); // 内部不画
    assert_eq!(at(0, 10), Color::GREEN); // 左边
}

#[test]
fn buffer_offset_area_coords() {
    // area 不是从 (0,0) 开始：模拟 PFB chunk（屏幕坐标 0..10 x 100..110）
    let area = Rect::new(0, 100, 10, 10);
    let mut px = vec![Color::BLACK; 100];
    let mut d = DrawBuf { pixels: &mut px, area, stride: 10 };
    d.fill_rect(Rect::new(0, 105, 10, 5), Color::RED, 255, area);
    assert_eq!(d.pixels[0], Color::BLACK); // 屏幕 y=100 行未画
    assert_eq!(d.pixels[5 * 10], Color::RED); // 屏幕 y=105 行
}

#[test]
fn rounded_corner_is_antialiased() {
    let (mut px, area) = buf(20, 20);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 20 };
    d.clear(Color::BLACK);
    d.fill_rounded(Rect::new(0, 0, 20, 20), 6, Color::WHITE, 255, area);
    let at = |x: usize, y: usize| d.pixels[y * 20 + x];
    // 角区（0..6）应存在部分混合像素（既非全黑也非全白）
    let mut partial = 0;
    for y in 0..6 {
        for x in 0..6 {
            let v = at(x, y).r;
            if v > 0 && v < 255 {
                partial += 1;
            }
        }
    }
    assert!(partial > 0, "圆角边缘应有半透明过渡像素");
    assert_eq!(at(0, 0), Color::BLACK); // 角点仍完全切掉
    assert_eq!(at(10, 10), Color::WHITE); // 中心不透明
}

#[test]
fn fill_circle_basic_and_aa_edge() {
    let (mut px, area) = buf(20, 20);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 20 };
    d.clear(Color::BLACK);
    d.fill_circle(qingui::Point { x: 10, y: 10 }, 5, Color::WHITE, 255, area);
    let at = |x: usize, y: usize| d.pixels[y * 20 + x];
    assert_eq!(at(10, 10), Color::WHITE); // 圆心
    assert_eq!(at(10, 14), Color::WHITE); // 半径内
    assert_eq!(at(3, 3), Color::BLACK); // 圆外（dist≈9.9 > 5）
    // 边缘存在半透明过渡
    let mut partial = false;
    for y in 4..17 {
        for x in 4..17 {
            let v = at(x, y).r;
            if v > 0 && v < 255 {
                partial = true;
            }
        }
    }
    assert!(partial, "圆盘边缘应有抗锯齿过渡");
}

#[test]
fn draw_circle_ring_hollow_center() {
    let (mut px, area) = buf(20, 20);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 20 };
    d.clear(Color::BLACK);
    d.draw_circle(qingui::Point { x: 10, y: 10 }, 5, 2, Color::GREEN, 255, area);
    let at = |x: usize, y: usize| d.pixels[y * 20 + x];
    assert_eq!(at(10, 10), Color::BLACK); // 环内空心
    assert_eq!(at(10, 14), Color::GREEN); // 环带（dist=4，在 3..5 环内）
    assert_eq!(at(3, 3), Color::BLACK); // 环外
}

#[test]
fn draw_arc_quarter_pie() {
    let (mut px, area) = buf(20, 20);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 20 };
    d.clear(Color::BLACK);
    // 0°..90°（右下象限）扇形
    d.draw_arc(qingui::Point { x: 10, y: 10 }, 5, 5, 0, 90, Color::RED, 255, area);
    let at = |x: usize, y: usize| d.pixels[y * 20 + x];
    assert_eq!(at(13, 13), Color::RED); // 右下 45°：在弧内
    assert_eq!(at(7, 7), Color::BLACK); // 左上：弧外
    assert_eq!(at(13, 7), Color::BLACK); // 右上：弧外
    assert_eq!(at(7, 13), Color::BLACK); // 左下：弧外
    assert_eq!(at(14, 11), Color::RED); // 接近 0° 方向：弧内
}

#[test]
fn draw_arc_full_sweep_equals_ring() {
    let (mut px, area) = buf(20, 20);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 20 };
    d.clear(Color::BLACK);
    d.draw_arc(qingui::Point { x: 10, y: 10 }, 5, 2, 0, 360, Color::GREEN, 255, area);
    let at = |x: usize, y: usize| d.pixels[y * 20 + x];
    assert_eq!(at(10, 14), Color::GREEN);
    assert_eq!(at(10, 6), Color::GREEN);
    assert_eq!(at(14, 10), Color::GREEN);
    assert_eq!(at(10, 10), Color::BLACK); // 环内空心
}

#[test]
fn draw_arc_wraparound_sweep() {
    let (mut px, area) = buf(20, 20);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 20 };
    d.clear(Color::BLACK);
    // 270°..90°（跨过 0° 的右半圆扇形，sweep=180）
    d.draw_arc(qingui::Point { x: 10, y: 10 }, 5, 5, 270, 90, Color::RED, 255, area);
    let at = |x: usize, y: usize| d.pixels[y * 20 + x];
    assert_eq!(at(13, 10), Color::RED); // 正右（0° 方向）：弧内
    assert_eq!(at(7, 10), Color::BLACK); // 正左（180° 方向）：弧外
}


#[test]
fn rgb565_roundtrip() {
    use qingui::Color;
    // 纯色端点
    assert_eq!(Color::from_rgb565(0xF800), Color::rgb(255, 0, 0));
    assert_eq!(Color::from_rgb565(0x07E0), Color::rgb(0, 255, 0));
    assert_eq!(Color::from_rgb565(0x001F), Color::rgb(0, 0, 255));
    assert_eq!(Color::from_rgb565(0xFFFF), Color::rgb(255, 255, 255));
    assert_eq!(Color::from_rgb565(0x0000), Color::rgb(0, 0, 0));
    // 全量往返不丢位
    for v in [0x0001u16, 0x1234, 0x7BEF, 0x8C51, 0xFFFE] {
        assert_eq!(Color::from_rgb565(v).to_rgb565(), v);
    }
}

#[test]
fn blit565_pixels_clip_and_opa() {
    use qingui::draw::DrawBuf;
    use qingui::{Color, Rect};
    // 2x2 图:红 绿 / 蓝 白(565 小端字节序)
    let data: [u8; 8] = [0x00, 0xF8, 0xE0, 0x07, 0x1F, 0x00, 0xFF, 0xFF];
    let mut buf = [Color::rgb(0, 0, 0); 16];
    {
        let mut d = DrawBuf { pixels: &mut buf, area: Rect::new(0, 0, 4, 4), stride: 4 };
        d.blit565(1, 1, 2, 2, &data, 255, Rect::new(0, 0, 4, 4));
    }
    assert_eq!(buf[1 * 4 + 1], Color::rgb(255, 0, 0));
    assert_eq!(buf[1 * 4 + 2], Color::rgb(0, 255, 0));
    assert_eq!(buf[2 * 4 + 1], Color::rgb(0, 0, 255));
    assert_eq!(buf[2 * 4 + 2], Color::rgb(255, 255, 255));
    // clip 裁剪:只允许左列
    let mut buf2 = [Color::rgb(0, 0, 0); 16];
    {
        let mut d = DrawBuf { pixels: &mut buf2, area: Rect::new(0, 0, 4, 4), stride: 4 };
        d.blit565(1, 1, 2, 2, &data, 255, Rect::new(0, 0, 2, 4));
    }
    assert_eq!(buf2[1 * 4 + 1], Color::rgb(255, 0, 0));
    assert_eq!(buf2[1 * 4 + 2], Color::rgb(0, 0, 0)); // 被裁掉
    // opa=0 不写;data 不足不画不 panic
    let mut buf3 = [Color::rgb(1, 2, 3); 4];
    {
        let mut d = DrawBuf { pixels: &mut buf3, area: Rect::new(0, 0, 2, 2), stride: 2 };
        d.blit565(0, 0, 2, 2, &data, 0, Rect::new(0, 0, 2, 2));
        d.blit565(0, 0, 4, 4, &data, 255, Rect::new(0, 0, 2, 2)); // 长度不足
    }
    assert_eq!(buf3, [Color::rgb(1, 2, 3); 4]);
}
