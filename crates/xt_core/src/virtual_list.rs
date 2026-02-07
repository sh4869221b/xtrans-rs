#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VirtualWindow {
    pub start: usize,
    pub end: usize,
    pub top_pad: f32,
    pub bottom_pad: f32,
}

impl VirtualWindow {
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }
}

pub fn virtual_window(
    total: usize,
    item_height: f32,
    viewport_height: f32,
    scroll_offset: f32,
    overscan: usize,
) -> VirtualWindow {
    if total == 0 {
        return VirtualWindow {
            start: 0,
            end: 0,
            top_pad: 0.0,
            bottom_pad: 0.0,
        };
    }

    let item_height = item_height.max(1.0);
    let viewport_height = viewport_height.max(item_height);
    let max_start = total.saturating_sub(1);
    let mut start = (scroll_offset.max(0.0) / item_height).floor() as usize;
    if start > max_start {
        start = max_start;
    }

    let visible = (viewport_height / item_height).ceil() as usize + 1;
    let safe_overscan = overscan.min(total);
    let start = start.saturating_sub(safe_overscan);
    let end = (start + visible + safe_overscan * 2).min(total);
    let top_pad = start as f32 * item_height;
    let bottom_pad = total.saturating_sub(end) as f32 * item_height;

    VirtualWindow {
        start,
        end,
        top_pad,
        bottom_pad,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_ui_002_virtual_window_10k() {
        let total = 10_000;
        let item_height = 32.0;
        let viewport_height = 480.0;

        let window = virtual_window(total, item_height, viewport_height, 0.0, 8);
        assert_eq!(window.start, 0);
        assert!(window.end <= total);
        let visible = (viewport_height / item_height).ceil() as usize + 1;
        assert!(window.len() <= visible + 8 * 2);

        let mid = virtual_window(total, item_height, viewport_height, 32.0 * 5000.0, 8);
        assert!(mid.start <= 5000);
        assert!(mid.end <= total);

        let end = virtual_window(total, item_height, viewport_height, 32.0 * total as f32, 8);
        assert!(end.end <= total);
        assert!(end.start <= end.end);
    }
}
