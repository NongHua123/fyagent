#[path = "../src/window_layout.rs"]
mod window_layout;

use window_layout::{
    clamp_window_geometry, default_size, effective_minimum_size, layout_mode, LayoutMode,
    LogicalWorkArea, WindowGeometry, DEFAULT_HEIGHT, DEFAULT_WIDTH, LAYOUT_VERSION,
    TARGET_MIN_HEIGHT, TARGET_MIN_WIDTH,
};

fn work_area(width: f64, height: f64) -> LogicalWorkArea {
    LogicalWorkArea {
        x: 0.0,
        y: 0.0,
        width,
        height,
    }
}

#[test]
fn uses_the_versioned_normal_policy_when_workspace_fits() {
    let area = work_area(1600.0, 1000.0);

    assert_eq!(LAYOUT_VERSION, 2);
    assert_eq!(layout_mode(area.width), LayoutMode::Normal);
    assert_eq!(effective_minimum_size(area).width, TARGET_MIN_WIDTH);
    assert_eq!(effective_minimum_size(area).height, TARGET_MIN_HEIGHT);
    assert_eq!(default_size(area).width, DEFAULT_WIDTH);
    assert_eq!(default_size(area).height, DEFAULT_HEIGHT);
}

#[test]
fn constrains_minimum_size_to_the_current_work_area() {
    let area = work_area(1000.0, 650.0);

    assert_eq!(layout_mode(area.width), LayoutMode::Constrained);
    assert_eq!(effective_minimum_size(area).width, 900.0);
    assert_eq!(effective_minimum_size(area).height, 585.0);
    assert_eq!(default_size(area).width, 900.0);
    assert_eq!(default_size(area).height, 585.0);
}

#[test]
fn clamps_invalid_and_off_screen_saved_geometry_without_dropping_maximized() {
    let restored = clamp_window_geometry(
        WindowGeometry {
            x: -900.0,
            y: 900.0,
            width: 6000.0,
            height: -1.0,
            maximized: true,
        },
        LogicalWorkArea {
            x: 100.0,
            y: 50.0,
            width: 1600.0,
            height: 1000.0,
        },
    );

    assert_eq!(restored.x, 100.0);
    assert_eq!(restored.y, 350.0);
    assert_eq!(restored.width, 1440.0);
    assert_eq!(restored.height, DEFAULT_HEIGHT);
    assert!(restored.maximized);
}
