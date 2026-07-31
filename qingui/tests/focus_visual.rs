use qingui::display::Flush;
use qingui::widgets::list::ListBuilder;
use qingui::widgets::obj::ObjBuilder;
use qingui::widgets::slider::SliderBuilder;
use qingui::widgets::switch::SwitchBuilder;
use qingui::{Color, Rect, Ui};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct RecFlush {
    chunks: Vec<(Rect, Vec<Color>)>,
}
struct SharedFlush(Rc<RefCell<RecFlush>>);
impl Flush for SharedFlush {
    fn flush(&mut self, area: Rect, pixels: &[Color]) {
        self.0.borrow_mut().chunks.push((area, pixels.to_vec()));
    }
}

fn px(rec: &Rc<RefCell<RecFlush>>, x: i32, y: i32) -> Color {
    let chunks = &rec.borrow().chunks;
    // 反向查找：后渲染的 chunk 覆盖先渲染的
    for (area, buf) in chunks.iter().rev() {
        if x >= area.x && x < area.right() && y >= area.y && y < area.bottom() {
            return buf[((y - area.y) * area.w + (x - area.x)) as usize];
        }
    }
    panic!("pixel not flushed");
}

fn setup() -> (Ui, Rc<RefCell<RecFlush>>) {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(160, 120, 120);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let mut bg = qingui::style::Style::default();
    bg.bg_color = Some(Color::BLACK);
    let scr = ui.screen();
    scr.set_style(&mut ui, bg);
    (ui, rec)
}

#[test]
fn slider_shows_focus_border() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let s = SliderBuilder::new(0, 100).build(&mut ui, scr);
    s.set_pos(&mut ui, 10, 10);
    s.group_add(&mut ui); // 成为焦点
    ui.render();
    // 聚焦态：白色边框，轨道顶边中点
    assert_eq!(px(&rec, 60, 10), Color::WHITE);
}

#[test]
fn moving_container_repaints_children_old_area() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let parent = ObjBuilder::new().build(&mut ui, scr);
    parent.set_pos(&mut ui, 10, 10);
    parent.set_size(&mut ui, 20, 20);
    let child = ObjBuilder::new().build(&mut ui, parent);
    child.set_pos(&mut ui, -10, 0); // 子元素超出父左边界
    child.set_size(&mut ui, 10, 10);
    let mut s = qingui::style::Style::default();
    s.bg_color = Some(Color::RED);
    child.set_style(&mut ui, s);
    ui.render();
    assert_eq!(px(&rec, 5, 15), Color::RED); // 子元素旧位置
    parent.set_pos(&mut ui, 40, 10); // 移动父容器
    ui.render();
    assert_eq!(px(&rec, 5, 15), Color::BLACK); // 旧区域必须重绘（无残影）
    assert_eq!(px(&rec, 35, 15), Color::RED); // 新位置
}

#[test]
fn moving_slider_repaints_knob_overflow() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let s = SliderBuilder::new(0, 100).build(&mut ui, scr);
    s.set_pos(&mut ui, 10, 10);
    s.set_value(&mut ui, 0); // 旋钮在最左，溢出到 x 6..14
    ui.render();
    assert_eq!(px(&rec, 7, 16), Color::WHITE); // 旋钮溢出区旧位置
    s.set_pos(&mut ui, 40, 10); // 移动滑块（布局动画同款路径）
    ui.render();
    assert_eq!(px(&rec, 7, 16), Color::BLACK); // 溢出区旧像素必须清除
}

#[test]
fn switch_shows_focus_border() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let sw = SwitchBuilder::new().build(&mut ui, scr);
    sw.set_pos(&mut ui, 10, 10);
    sw.group_add(&mut ui);
    ui.render();
    // 聚焦态：白色边框，轨道顶边中点
    assert_eq!(px(&rec, 30, 10), Color::WHITE);
}

#[test]
fn slider_knob_overflow_area_redrawn_on_move() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let s = SliderBuilder::new(0, 100).build(&mut ui, scr);
    s.set_pos(&mut ui, 10, 10);
    ui.render();
    // 初始 knob 在 x 6..14, y 8..24（轨道上方溢出 2px）
    assert_eq!(px(&rec, 10, 8), Color::WHITE);
    s.set_value(&mut ui, 50);
    ui.render();
    // 旧 knob 溢出区域被重绘为背景（不留残影）
    assert_eq!(px(&rec, 10, 8), Color::BLACK);
    // 新 knob 位置（kx = 10+50 = 60，knob x 56..64）
    assert_eq!(px(&rec, 60, 8), Color::WHITE);
}

#[test]
fn list_highlight_respects_rounded_corner() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let l = ListBuilder::new(&["a", "b", "c"]).build(&mut ui, scr);
    l.set_pos(&mut ui, 10, 10);
    ui.render();
    // 首行高亮的左上角（圆角区内）不应是高亮色
    assert_ne!(px(&rec, 10, 12), Color::rgb(50, 70, 120));
    // 首行内部（边框之下）是高亮色
    assert_eq!(px(&rec, 60, 12), Color::rgb(50, 70, 120));
}

#[test]
fn list_ghost_fully_cleared_after_fade() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let l = ListBuilder::new(&["a", "b", "c"]).build(&mut ui, scr);
    l.set_pos(&mut ui, 10, 10);
    l.list_select(&mut ui, 2);
    ui.render();
    assert!(l.list_remove(&mut ui)); // 删除 "c"，ghost 渐隐
    ui.tick_inc(500); // 超过 FX_DUR
    ui.timer_handler();
    // ghost 所在行（行 2）区域应恢复列表背景色，无文字残留
    for x in 14..40 {
        assert_eq!(px(&rec, x, 50), Color::rgb(34, 34, 44), "x={}", x);
    }
}
