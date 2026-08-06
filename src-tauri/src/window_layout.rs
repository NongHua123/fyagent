//! Pure window-layout policy shared conceptually with the renderer contract.
//!
//! This module intentionally has no Tauri dependency: host startup code can
//! call it while the main window is still hidden, before applying geometry and
//! restoring maximization.

pub const LAYOUT_VERSION: u32 = 2;
pub const TARGET_MIN_WIDTH: f64 = 1152.0;
pub const TARGET_MIN_HEIGHT: f64 = 640.0;
pub const DEFAULT_WIDTH: f64 = 1232.0;
pub const DEFAULT_HEIGHT: f64 = 700.0;
pub const MAXIMUM_WORK_AREA_SHARE: f64 = 0.9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Normal,
    Constrained,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogicalSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogicalWorkArea {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowGeometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub maximized: bool,
}

fn finite_positive(value: f64, fallback: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

fn finite_coordinate(value: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

fn clamp(value: f64, minimum: f64, maximum: f64) -> f64 {
    value.max(minimum).min(maximum)
}

fn maximum_visible_dimension(dimension: f64) -> f64 {
    (finite_positive(dimension, 1.0) * MAXIMUM_WORK_AREA_SHARE)
        .floor()
        .max(1.0)
}

pub fn normalize_work_area(work_area: LogicalWorkArea) -> LogicalWorkArea {
    LogicalWorkArea {
        x: finite_coordinate(work_area.x, 0.0),
        y: finite_coordinate(work_area.y, 0.0),
        width: finite_positive(work_area.width, DEFAULT_WIDTH),
        height: finite_positive(work_area.height, DEFAULT_HEIGHT),
    }
}

pub fn layout_mode(work_area_width: f64) -> LayoutMode {
    if work_area_width >= TARGET_MIN_WIDTH {
        LayoutMode::Normal
    } else {
        LayoutMode::Constrained
    }
}

pub fn effective_minimum_size(work_area: LogicalWorkArea) -> LogicalSize {
    let work_area = normalize_work_area(work_area);
    LogicalSize {
        width: TARGET_MIN_WIDTH.min(maximum_visible_dimension(work_area.width)),
        height: TARGET_MIN_HEIGHT.min(maximum_visible_dimension(work_area.height)),
    }
}

pub fn default_size(work_area: LogicalWorkArea) -> LogicalSize {
    let work_area = normalize_work_area(work_area);
    let minimum = effective_minimum_size(work_area);
    LogicalSize {
        width: clamp(
            DEFAULT_WIDTH,
            minimum.width,
            maximum_visible_dimension(work_area.width),
        ),
        height: clamp(
            DEFAULT_HEIGHT,
            minimum.height,
            maximum_visible_dimension(work_area.height),
        ),
    }
}

pub fn clamp_window_geometry(saved: WindowGeometry, work_area: LogicalWorkArea) -> WindowGeometry {
    let work_area = normalize_work_area(work_area);
    let minimum = effective_minimum_size(work_area);
    let defaults = default_size(work_area);
    let maximum_width = maximum_visible_dimension(work_area.width);
    let maximum_height = maximum_visible_dimension(work_area.height);
    let width = clamp(
        finite_positive(saved.width, defaults.width),
        minimum.width,
        maximum_width,
    );
    let height = clamp(
        finite_positive(saved.height, defaults.height),
        minimum.height,
        maximum_height,
    );

    WindowGeometry {
        x: clamp(
            finite_coordinate(saved.x, work_area.x),
            work_area.x,
            work_area.x + work_area.width - width,
        ),
        y: clamp(
            finite_coordinate(saved.y, work_area.y),
            work_area.y,
            work_area.y + work_area.height - height,
        ),
        width,
        height,
        maximized: saved.maximized,
    }
}
