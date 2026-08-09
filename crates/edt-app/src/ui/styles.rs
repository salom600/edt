//! Misc style helpers.

use egui::Color32;

/// Standard clip label color based on track kind.
pub fn clip_label_color(is_video: bool) -> Color32 {
    if is_video {
        Color32::from_rgb(180, 215, 255)
    } else {
        Color32::from_rgb(200, 230, 180)
    }
}

/// Convert a 0..7 label index into a color.
pub fn label_color(idx: u8) -> Color32 {
    const COLORS: [Color32; 8] = [
        Color32::from_rgb(96, 165, 250),  // blue
        Color32::from_rgb(167, 139, 250), // purple
        Color32::from_rgb(244, 114, 182), // pink
        Color32::from_rgb(248, 113, 113), // red
        Color32::from_rgb(251, 146, 60),  // orange
        Color32::from_rgb(250, 204, 21),  // yellow
        Color32::from_rgb(132, 204, 22),  // green
        Color32::from_rgb(20, 184, 166),  // teal
    ];
    COLORS[(idx as usize) % COLORS.len()]
}
