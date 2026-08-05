use alloc::vec::Vec;
use crate::geometry::Rect;

/// A bounded queue of dirty (needs-repaint) screen rectangles, clipped to the screen
/// and merged with neighbors so overlapping regions are redrawn only once.
pub struct DirtyQueue {
    rects: Vec<Rect>,
    cap: usize,
    screen: Rect,
}

impl DirtyQueue {
    /// Creates a queue that clips all rects to `screen` and flushes to a full-screen
    /// rect once more than `cap` rects are pending.
    pub fn new(screen: Rect, cap: usize) -> Self {
        Self { rects: Vec::new(), cap: cap.max(1), screen }
    }

    /// Marks `r` as dirty, merging it with intersecting or adjacent pending rects.
    pub fn add(&mut self, r: Rect) {
        let Some(r) = r.intersect(&self.screen) else { return };
        if r.is_empty() {
            return;
        }
        if self.rects.len() == 1 && self.rects[0] == self.screen {
            return; // already full screen
        }
        // Iteratively merge rects that intersect or share an edge (sharing: intersect after 1px expansion)
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

    /// Takes all pending dirty rects, leaving the queue empty.
    pub fn take(&mut self) -> Vec<Rect> {
        core::mem::take(&mut self.rects)
    }

    /// Returns `true` if there are no pending dirty rects.
    pub fn is_empty(&self) -> bool {
        self.rects.is_empty()
    }
}
