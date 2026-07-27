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
