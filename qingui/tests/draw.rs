use qingui::draw::DrawBuf;
use qingui::{Color, Rect};

fn buf(w: i32, h: i32) -> (Vec<Color>, Rect) {
    (vec![Color::BLACK; (w * h) as usize], Rect::new(0, 0, w, h))
}

/// Build a buffer from ASCII art: `.` = BLACK, `#` = WHITE. Every row must be the
/// same width; trailing whitespace is ignored. Lets tests import the expected shape
/// visually (a 32x8 screen looks like this):
///
/// ```text
/// ................................
/// ....................#...........
/// .....................#..........
/// ......................#.........
/// .......................#........
/// ........................#.......
/// ................................
/// ................................
/// ```
fn ascii_buf(art: &str) -> (Vec<Color>, Rect) {
    let lines: Vec<&str> = art.lines().map(str::trim_end).collect();
    assert!(!lines.is_empty(), "empty ascii art");
    let w = lines[0].chars().count() as i32;
    let h = lines.len() as i32;
    let mut px = vec![Color::BLACK; (w * h) as usize];
    for (y, line) in lines.iter().enumerate() {
        assert_eq!(line.chars().count() as i32, w, "row {y} has a different width");
        for (x, ch) in line.chars().enumerate() {
            match ch {
                '.' => {}
                '#' => px[(y * w as usize) + x] = Color::WHITE,
                other => panic!("unexpected char {other:?} in ascii art"),
            }
        }
    }
    (px, Rect::new(0, 0, w, h))
}

/// Render a buffer back to ASCII art (`.` = BLACK, `#` = anything else), so a shape
/// can be asserted against an expected bitmap or eyeballed in a failure message.
fn to_ascii(d: &DrawBuf) -> String {
    let mut out = String::with_capacity((d.area.w * (d.area.h + 1)) as usize);
    for y in 0..d.area.h {
        if y > 0 {
            out.push('\n');
        }
        for x in 0..d.area.w {
            let c = d.pixels[(y * d.stride + x) as usize];
            out.push(if c == Color::BLACK { '.' } else { '#' });
        }
    }
    out
}

#[test]
fn ascii_buf_roundtrip() {
    // A buffer imported from text renders back to the identical bitmap.
    let art = "\
....#....
...#..#..
#..#..#.#
.........";
    let (mut px, area) = ascii_buf(art);
    let d = DrawBuf { pixels: &mut px, area, stride: area.w };
    assert_eq!(to_ascii(&d), art);
}

#[test]
fn draw_onto_ascii_background() {
    // Import a pre-filled canvas from text, then draw a shape on top of it.
    let (mut px, area) = ascii_buf("\
########
########
########
########
########
########
########
########");
    let mut d = DrawBuf { pixels: &mut px, area, stride: 8 };
    d.fill_rect(Rect::new(2, 2, 4, 4), Color::BLACK, 255, area);
    assert_eq!(to_ascii(&d), "\
########
########
##....##
##....##
##....##
##....##
########
########");
}

#[test]
fn fill_rect_basic() {
    let (mut px, area) = buf(10, 10);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 10 };
    d.fill_rect(Rect::new(2, 2, 3, 3), Color::RED, 255, area);
    assert_eq!(to_ascii(&d), "\
..........
..........
..###.....
..###.....
..###.....
..........
..........
..........
..........
..........");
}

#[test]
fn fill_rect_clipped() {
    let (mut px, area) = buf(10, 10);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 10 };
    let clip = Rect::new(0, 0, 3, 10);
    d.fill_rect(Rect::new(0, 0, 10, 10), Color::RED, 255, clip);
    assert_eq!(to_ascii(&d), "\
###.......
###.......
###.......
###.......
###.......
###.......
###.......
###.......
###.......
###.......");
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
    assert_eq!(to_ascii(&d), "\
....############....
..################..
.##################.
.##################.
####################
####################
####################
####################
####################
####################
####################
####################
####################
####################
####################
####################
.##################.
.##################.
..################..
....############....");
}

#[test]
fn draw_border_ring() {
    let (mut px, area) = buf(20, 20);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 20 };
    d.draw_border(Rect::new(0, 0, 20, 20), 2, 0, Color::GREEN, 255, area);
    assert_eq!(to_ascii(&d), "\
####################
####################
##................##
##................##
##................##
##................##
##................##
##................##
##................##
##................##
##................##
##................##
##................##
##................##
##................##
##................##
##................##
##................##
####################
####################");
}

#[test]
fn buffer_offset_area_coords() {
    // area does not start at (0,0): simulates a PFB chunk (screen coords 0..10 x 100..110)
    let area = Rect::new(0, 100, 10, 10);
    let mut px = vec![Color::BLACK; 100];
    let mut d = DrawBuf { pixels: &mut px, area, stride: 10 };
    d.fill_rect(Rect::new(0, 105, 10, 5), Color::RED, 255, area);
    assert_eq!(d.pixels[0], Color::BLACK); // screen y=100 row not drawn
    assert_eq!(d.pixels[5 * 10], Color::RED); // screen y=105 row
}

#[test]
fn rounded_corner_is_antialiased() {
    let (mut px, area) = buf(20, 20);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 20 };
    d.clear(Color::BLACK);
    d.fill_rounded(Rect::new(0, 0, 20, 20), 6, Color::WHITE, 255, area);
    let at = |x: usize, y: usize| d.pixels[y * 20 + x];
    // The corner region (0..6) should contain some partially blended pixels (neither fully black nor fully white)
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
    assert_eq!(to_ascii(&d), "\
....############....
..################..
.##################.
.##################.
####################
####################
####################
####################
####################
####################
####################
####################
####################
####################
####################
####################
.##################.
.##################.
..################..
....############....");
}

#[test]
fn fill_circle_basic_and_aa_edge() {
    let (mut px, area) = buf(20, 20);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 20 };
    d.clear(Color::BLACK);
    d.fill_circle(qingui::Point { x: 10, y: 10 }, 5, Color::WHITE, 255, area);
    let at = |x: usize, y: usize| d.pixels[y * 20 + x];
    assert_eq!(to_ascii(&d), "\
....................
....................
....................
....................
....................
........#####.......
.......#######......
......#########.....
.....###########....
.....###########....
.....###########....
.....###########....
.....###########....
......#########.....
.......#######......
........#####.......
....................
....................
....................
....................");
    // The edge has a semi-transparent transition
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
    assert_eq!(to_ascii(&d), "\
....................
....................
....................
....................
....................
........#####.......
.......#######......
......#########.....
.....####...####....
.....###.....###....
.....###.....###....
.....###.....###....
.....####...####....
......#########.....
.......#######......
........#####.......
....................
....................
....................
....................");
}

#[test]
fn draw_arc_quarter_pie() {
    let (mut px, area) = buf(20, 20);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 20 };
    d.clear(Color::BLACK);
    // 0°..90° (bottom-right quadrant) pie sector
    d.draw_arc(qingui::Point { x: 10, y: 10 }, 5, 5, 0, 90, Color::RED, 255, area);
    assert_eq!(to_ascii(&d), "\
....................
....................
....................
....................
....................
....................
....................
....................
....................
....................
..........######....
..........######....
..........######....
..........#####.....
..........####......
..........###.......
....................
....................
....................
....................");
}

#[test]
fn draw_arc_full_sweep_equals_ring() {
    let (mut px, area) = buf(20, 20);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 20 };
    d.clear(Color::BLACK);
    d.draw_arc(qingui::Point { x: 10, y: 10 }, 5, 2, 0, 360, Color::GREEN, 255, area);
    assert_eq!(to_ascii(&d), "\
....................
....................
....................
....................
....................
........#####.......
.......#######......
......#########.....
.....####...####....
.....###.....###....
.....###.....###....
.....###.....###....
.....####...####....
......#########.....
.......#######......
........#####.......
....................
....................
....................
....................");
}

#[test]
fn draw_arc_wraparound_sweep() {
    let (mut px, area) = buf(20, 20);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 20 };
    d.clear(Color::BLACK);
    // 270°..90° (right-half pie sector crossing 0°, sweep=180)
    d.draw_arc(qingui::Point { x: 10, y: 10 }, 5, 5, 270, 90, Color::RED, 255, area);
    assert_eq!(to_ascii(&d), "\
....................
....................
....................
....................
....................
..........###.......
..........####......
..........#####.....
..........######....
..........######....
..........######....
..........######....
..........######....
..........#####.....
..........####......
..........###.......
....................
....................
....................
....................");
}

#[test]
fn draw_line_diagonal_32x8() {
    // A 32x8 screen: '.' = 0, '#' = 1. The line (20,1)..(24,5) shows up as a
    // visible diagonal instead of five scattered one-pixel asserts.
    let (mut px, area) = buf(32, 8);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 32 };
    d.draw_line(qingui::Point { x: 20, y: 1 }, qingui::Point { x: 24, y: 5 }, 1, Color::RED, 255, area);
    assert_eq!(to_ascii(&d), "\
................................
....................#...........
.....................#..........
......................#.........
.......................#........
........................#.......
................................
................................");
}

#[test]
fn draw_line_horizontal() {
    let (mut px, area) = buf(8, 8);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 8 };
    d.draw_line(qingui::Point { x: 1, y: 3 }, qingui::Point { x: 6, y: 3 }, 1, Color::RED, 255, area);
    assert_eq!(to_ascii(&d), "\
........
........
........
.######.
........
........
........
........");
}

#[test]
fn draw_line_45deg_short() {
    // Short 45° diagonal (length 5) across an 8x8 buffer.
    let (mut px, area) = buf(8, 8);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 8 };
    d.draw_line(qingui::Point { x: 2, y: 1 }, qingui::Point { x: 6, y: 5 }, 1, Color::RED, 255, area);
    assert_eq!(to_ascii(&d), "\
........
..#.....
...#....
....#...
.....#..
......#.
........
........");
}

#[test]
fn draw_line_45deg_full() {
    // Full 45° diagonal spanning the whole 16x16 buffer.
    let (mut px, area) = buf(16, 16);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 16 };
    d.draw_line(qingui::Point { x: 0, y: 0 }, qingui::Point { x: 15, y: 15 }, 1, Color::RED, 255, area);
    assert_eq!(to_ascii(&d), "\
#...............
.#..............
..#.............
...#............
....#...........
.....#..........
......#.........
.......#........
........#.......
.........#......
..........#.....
...........#....
............#...
.............#..
..............#.
...............#");
}

#[test]
fn draw_line_steep() {
    // Steep (near-vertical) diagonal: dx=2, dy=7.
    let (mut px, area) = buf(16, 8);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 16 };
    d.draw_line(qingui::Point { x: 10, y: 0 }, qingui::Point { x: 12, y: 7 }, 1, Color::RED, 255, area);
    assert_eq!(to_ascii(&d), "\
..........#.....
..........#.....
...........#....
...........#....
...........#....
...........#....
............#...
............#...");
}

#[test]
fn draw_line_shallow() {
    // Shallow (near-horizontal) diagonal: dx=15, dy=2.
    let (mut px, area) = buf(16, 8);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 16 };
    d.draw_line(qingui::Point { x: 0, y: 4 }, qingui::Point { x: 15, y: 6 }, 1, Color::RED, 255, area);
    assert_eq!(to_ascii(&d), "\
................
................
................
................
####............
....########....
............####
................");
}

#[test]
fn draw_line_up_right() {
    // Negative-slope diagonal (going up to the right).
    let (mut px, area) = buf(16, 8);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 16 };
    d.draw_line(qingui::Point { x: 5, y: 7 }, qingui::Point { x: 11, y: 1 }, 1, Color::RED, 255, area);
    assert_eq!(to_ascii(&d), "\
................
...........#....
..........#.....
.........#......
........#.......
.......#........
......#.........
.....#..........");
}

#[test]
fn draw_line_long_diag() {
    // Long shallow diagonal (dx=27, dy=11) across a 32x16 buffer.
    let (mut px, area) = buf(32, 16);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 32 };
    d.draw_line(qingui::Point { x: 2, y: 2 }, qingui::Point { x: 29, y: 13 }, 1, Color::RED, 255, area);
    assert_eq!(to_ascii(&d), "\
................................
................................
..##............................
....##..........................
......###.......................
.........##.....................
...........###..................
..............##................
................##..............
..................###...........
.....................##.........
.......................###......
..........................##....
............................##..
................................
................................");
}

#[test]
fn rgb565_roundtrip() {
    use qingui::Color;
    // Solid-color endpoints
    assert_eq!(Color::from_rgb565(0xF800), Color::rgb(255, 0, 0));
    assert_eq!(Color::from_rgb565(0x07E0), Color::rgb(0, 255, 0));
    assert_eq!(Color::from_rgb565(0x001F), Color::rgb(0, 0, 255));
    assert_eq!(Color::from_rgb565(0xFFFF), Color::rgb(255, 255, 255));
    assert_eq!(Color::from_rgb565(0x0000), Color::rgb(0, 0, 0));
    // Full round-trip loses no bits
    for v in [0x0001u16, 0x1234, 0x7BEF, 0x8C51, 0xFFFE] {
        assert_eq!(Color::from_rgb565(v).to_rgb565(), v);
    }
}

#[test]
fn fill_rect_full_coverage_opa_255() {
    // Full-screen opaque fill must paint every pixel exactly (slice-fill path).
    let (mut px, area) = buf(5, 4);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 5 };
    d.fill_rect(area, Color::WHITE, 255, area);
    assert!(d.pixels.iter().all(|&c| c == Color::WHITE), "opaque fill must cover every pixel");
    assert_eq!(to_ascii(&d), "\
#####
#####
#####
#####");
}

#[test]
fn fill_rect_partial_opa_blends() {
    // Non-full opacity on a sub-rect blends over the black background.
    let (mut px, area) = buf(4, 3);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 4 };
    d.fill_rect(Rect::new(1, 1, 2, 2), Color::WHITE, 128, area);
    let at = |x: usize, y: usize| d.pixels[y * 4 + x];
    assert_eq!(at(1, 1), Color::BLACK.blend(Color::WHITE, 128), "blend arithmetic");
    // white over black at 128 opa -> mid-gray (~128)
    let mid = at(1, 1);
    assert!(mid.r > 60 && mid.r < 195, "mid-gray blend, got {}", mid.r);
    // corners untouched
    assert_eq!(at(0, 0), Color::BLACK);
}

#[test]
fn blit565_pixels_clip_and_opa() {
    use qingui::draw::DrawBuf;
    use qingui::{Color, Rect};
    // 2x2 image: red green / blue white (565 little-endian byte order)
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
    // clip: only the left column is allowed
    let mut buf2 = [Color::rgb(0, 0, 0); 16];
    {
        let mut d = DrawBuf { pixels: &mut buf2, area: Rect::new(0, 0, 4, 4), stride: 4 };
        d.blit565(1, 1, 2, 2, &data, 255, Rect::new(0, 0, 2, 4));
    }
    assert_eq!(buf2[1 * 4 + 1], Color::rgb(255, 0, 0));
    assert_eq!(buf2[1 * 4 + 2], Color::rgb(0, 0, 0)); // clipped off
    // opa=0 writes nothing; insufficient data draws nothing and does not panic
    let mut buf3 = [Color::rgb(1, 2, 3); 4];
    {
        let mut d = DrawBuf { pixels: &mut buf3, area: Rect::new(0, 0, 2, 2), stride: 2 };
        d.blit565(0, 0, 2, 2, &data, 0, Rect::new(0, 0, 2, 2));
        d.blit565(0, 0, 4, 4, &data, 255, Rect::new(0, 0, 2, 2)); // insufficient length
    }
    assert_eq!(buf3, [Color::rgb(1, 2, 3); 4]);
}
