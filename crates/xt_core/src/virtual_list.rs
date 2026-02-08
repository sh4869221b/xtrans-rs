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
    show_rows_window(
        total,
        item_height,
        viewport_height,
        scroll_offset,
        overscan,
    )
}

pub fn show_rows_window(
    total: usize,
    item_height: f32,
    viewport_height: f32,
    scroll_offset: f32,
    overscan: usize,
) -> VirtualWindow {
    // Mirrors egui::ScrollArea::show_rows row range calculation.
    if total == 0 {
        return VirtualWindow {
            start: 0,
            end: 0,
            top_pad: 0.0,
            bottom_pad: 0.0,
        };
    }

    let item_height = item_height.max(1.0);
    let viewport_height = viewport_height.max(0.0);
    let scroll_offset = scroll_offset.max(0.0);

    let visible_start = (scroll_offset / item_height).floor() as usize;
    let visible_end = ((scroll_offset + viewport_height) / item_height).ceil() as usize;
    let visible_start = visible_start.min(total);
    let visible_end = visible_end.min(total);

    let safe_overscan = overscan.min(total);
    let start = visible_start.saturating_sub(safe_overscan);
    let end = (visible_end + safe_overscan).min(total);
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

        let window = show_rows_window(total, item_height, viewport_height, 0.0, 8);
        assert_eq!(window.start, 0);
        assert!(window.end <= total);
        let visible_end = ((viewport_height / item_height).ceil() as usize).min(total);
        assert!(window.len() <= visible_end + 8);

        let mid = show_rows_window(total, item_height, viewport_height, 32.0 * 5000.0, 8);
        assert!(mid.start <= 5000);
        assert!(mid.end <= total);

        let end = show_rows_window(total, item_height, viewport_height, 32.0 * total as f32, 8);
        assert!(end.end <= total);
        assert!(end.start <= end.end);
    }
}
