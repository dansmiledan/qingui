use crate::arena::ObjRef;

/// 焦点簿记：计算焦点应移动到的目标索引（纯函数，副作用由 Ui 执行）。
/// 语义与旧 Ui::group_focus_next/prev 完全一致：
/// 空组 → None；base = focused.unwrap_or(0)；从 base 沿 dir（±1）步进
/// 1..=len，跳过 !valid，环绕取模（rem_euclid）；全不可选 → None。
pub(crate) fn step(
    group: &[ObjRef],
    focused: Option<usize>,
    dir: i32,
    valid: impl Fn(ObjRef) -> bool,
) -> Option<usize> {
    if group.is_empty() {
        return None;
    }
    let base = focused.unwrap_or(0);
    let len = group.len();
    for k in 1..=len {
        let idx = ((base as i64 + dir as i64 * k as i64).rem_euclid(len as i64)) as usize;
        if valid(group[idx]) {
            return Some(idx);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn obj(i: u32) -> ObjRef { ObjRef { index: i, generation: 0 } }

    #[test]
    fn empty_group_returns_none() {
        assert_eq!(step(&[], Some(0), 1, |_| true), None);
    }

    #[test]
    fn next_wraps_around() {
        let g = vec![obj(0), obj(1), obj(2)];
        // focused=2，Next(+1) → 环绕到 0
        assert_eq!(step(&g, Some(2), 1, |_| true), Some(0));
        // focused=0，Prev(-1) → 环绕到 2
        assert_eq!(step(&g, Some(0), -1, |_| true), Some(2));
    }

    #[test]
    fn skips_invalid() {
        let g = vec![obj(0), obj(1), obj(2)];
        // focused=0，Next(+1)：obj1 不可选 → 跳过到 obj2
        let valid = |o: ObjRef| o.index != 1;
        assert_eq!(step(&g, Some(0), 1, valid), Some(2));
        // 全不可选 → None
        assert_eq!(step(&g, Some(0), 1, |_| false), None);
    }

    #[test]
    fn none_focused_starts_at_zero() {
        let g = vec![obj(0), obj(1)];
        assert_eq!(step(&g, None, 1, |_| true), Some(1));
    }
}
