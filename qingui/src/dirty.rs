use alloc::vec::Vec;
use crate::geometry::Rect;

pub struct DirtyQueue {
    rects: Vec<Rect>,
    cap: usize,
    screen: Rect,
}

impl DirtyQueue {
    pub fn new(screen: Rect, cap: usize) -> Self {
        Self { rects: Vec::new(), cap: cap.max(1), screen }
    }

    pub fn add(&mut self, r: Rect) {
        let Some(r) = r.intersect(&self.screen) else { return };
        if r.is_empty() {
            return;
        }
        if self.rects.len() == 1 && self.rects[0] == self.screen {
            return; // 已是全屏
        }
        // 与相交或共边相邻的矩形迭代合并（共边：膨胀 1px 后相交）
        let mut cur = r;
        loop {
            let mut merged = false;
            let grown = Rect::new(cur.x - 1, cur.y - 1, cur.w + 2, cur.h + 2);
            let mut i = 0;
            while i < self.rects.len() {
                if grown.intersects(&self.rects[i]) {
                    cur = cur.union(&self.rects.remove(i));
                    merged = true;
                    break;
                }
                i += 1;
            }
            if !merged {
                break;
            }
        }
        self.rects.push(cur);
        if self.rects.len() > self.cap {
            self.rects.clear();
            self.rects.push(self.screen);
        }
    }

    pub fn take(&mut self) -> Vec<Rect> {
        core::mem::take(&mut self.rects)
    }

    pub fn is_empty(&self) -> bool {
        self.rects.is_empty()
    }
}
