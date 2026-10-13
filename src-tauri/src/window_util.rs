//! Helpers shared by the app's extra windows (YouTube media window, YouTube sign-in window, phase popup).

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, PhysicalPosition, PhysicalSize, Url, WebviewWindowBuilder, Wry};
use tauri_plugin_opener::OpenerExt;

use crate::{MAIN_WINDOW, lock};

/// Gap between a corner window and the work area edge (logical px).
pub const SCREEN_MARGIN: f64 = 16.0;
/// A remote page gets at most one link opened in the system browser per this interval (it cannot spam tabs).
const EXTERNAL_OPEN_EVERY: Duration = Duration::from_secs(2);

/// Hands an http(s) link that a remote-page window must not open itself to the system browser, at most once per
/// [`EXTERNAL_OPEN_EVERY`] per window (`last_opened` is that window's clock). `window` names it in the log; only the
/// host is logged (links may carry personal ids in the query).
pub fn open_link_externally(
    app: &AppHandle,
    last_opened: &Mutex<Option<Instant>>,
    url: &Url,
    window: &str,
) {
    if !matches!(url.scheme(), "http" | "https") {
        log::debug!("{window}: blocked a {} link", url.scheme());
        return;
    }
    {
        let mut last = lock(last_opened);
        if last.is_some_and(|at| at.elapsed() < EXTERNAL_OPEN_EVERY) {
            log::debug!("{window}: external link ignored (rate limit)");
            return;
        }
        *last = Some(Instant::now());
    }
    log::info!(
        "{window}: opening {} in the browser",
        url.host_str().unwrap_or("?")
    );
    let app = app.clone();
    let link = url.to_string();
    // Off the main thread: starting the browser can take a moment.
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(err) = app.opener().open_url(link, None::<&str>) {
            log::warn!("cannot open a link in the browser: {err}");
        }
    });
}

/// Where a window of a given logical size goes near the bottom-right corner of the primary monitor's
/// work area (above the taskbar).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CornerPlacement {
    /// Top-left corner in logical px (for `WebviewWindowBuilder::position`).
    pub logical: (f64, f64),
    /// The same corner in physical px of the primary monitor.
    pub physical: PhysicalPosition<i32>,
    /// The window size in physical px of the primary monitor.
    pub size: PhysicalSize<u32>,
}

/// Bottom-right placement for a window of `width` x `height` logical px, [`SCREEN_MARGIN`] from the edges.
/// `None` when the primary monitor is unknown.
pub fn bottom_right(app: &AppHandle, width: f64, height: f64) -> Option<CornerPlacement> {
    let monitor = app.primary_monitor().ok().flatten()?;
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    corner(
        (
            f64::from(area.position.x),
            f64::from(area.position.y),
            f64::from(area.size.width),
            f64::from(area.size.height),
        ),
        scale,
        width,
        height,
    )
}

/// Pure part of [`bottom_right`]: `area` is the physical work area `(x, y, width, height)`.
fn corner(
    area: (f64, f64, f64, f64),
    scale: f64,
    width: f64,
    height: f64,
) -> Option<CornerPlacement> {
    if !scale.is_finite() || scale <= 0.0 || !width.is_finite() || !height.is_finite() {
        return None;
    }
    let (left, top, area_width, area_height) = area;
    let width_px = (width * scale).round().max(1.0);
    let height_px = (height * scale).round().max(1.0);
    let margin_px = (SCREEN_MARGIN * scale).round();
    let x = (left + area_width - width_px - margin_px).max(left).round();
    let y = (top + area_height - height_px - margin_px).max(top).round();
    Some(CornerPlacement {
        logical: (x / scale, y / scale),
        physical: PhysicalPosition::new(to_i32(x), to_i32(y)),
        size: PhysicalSize::new(to_u32(width_px), to_u32(height_px)),
    })
}

/// WebView2 shares one browser process per data directory, and all its webviews must use the same
/// environment options, or creating the second webview fails. Every extra window copies the main
/// window's options from `tauri.conf.json`.
pub fn with_main_webview_options<'a>(
    app: &AppHandle,
    mut builder: WebviewWindowBuilder<'a, Wry, AppHandle>,
) -> WebviewWindowBuilder<'a, Wry, AppHandle> {
    if let Some(main) = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == MAIN_WINDOW)
    {
        if let Some(args) = &main.additional_browser_args {
            builder = builder.additional_browser_args(args);
        }
        builder = builder.browser_extensions_enabled(main.browser_extensions_enabled);
        #[cfg(windows)]
        if matches!(
            main.scroll_bar_style,
            tauri::utils::config::ScrollBarStyle::FluentOverlay
        ) {
            builder = builder.scroll_bar_style(tauri::webview::ScrollBarStyle::FluentOverlay);
        }
    }
    builder
}

/// Saturating `f64` → `i32` for already rounded pixel values.
pub fn to_i32(value: f64) -> i32 {
    if value.is_nan() {
        0
    } else {
        // Clamped into the target range first, so the cast is exact.
        value.clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
    }
}

/// Saturating `f64` → `u32` for already rounded pixel values.
pub fn to_u32(value: f64) -> u32 {
    if value.is_nan() {
        0
    } else {
        // Clamped into the target range first, so the cast is exact.
        value.clamp(0.0, f64::from(u32::MAX)) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corner_sits_above_the_taskbar_with_a_margin() {
        // 1920x1040 work area (taskbar at the bottom), 100 %.
        let placement = corner((0.0, 0.0, 1920.0, 1040.0), 1.0, 380.0, 150.0).expect("placement");
        assert_eq!(placement.physical, PhysicalPosition::new(1524, 874));
        assert_eq!(placement.size, PhysicalSize::new(380, 150));
        assert_eq!(placement.logical, (1524.0, 874.0));
    }

    #[test]
    fn corner_scales_with_dpi_and_offsets() {
        // Primary monitor at 150 %, work area starting at (100, 50).
        let placement =
            corner((100.0, 50.0, 2880.0, 1560.0), 1.5, 380.0, 150.0).expect("placement");
        assert_eq!(placement.size, PhysicalSize::new(570, 225));
        assert_eq!(placement.physical, PhysicalPosition::new(2386, 1361));
        assert!((placement.logical.0 - 2386.0 / 1.5).abs() < 1e-9);
    }

    #[test]
    fn corner_never_leaves_a_tiny_work_area() {
        let placement = corner((10.0, 20.0, 200.0, 100.0), 1.0, 380.0, 150.0).expect("placement");
        assert_eq!(placement.physical, PhysicalPosition::new(10, 20));
        assert!(corner((0.0, 0.0, 100.0, 100.0), 0.0, 1.0, 1.0).is_none());
        assert!(corner((0.0, 0.0, 100.0, 100.0), f64::NAN, 1.0, 1.0).is_none());
    }

    #[test]
    fn pixel_casts_saturate() {
        assert_eq!(to_i32(f64::NAN), 0);
        assert_eq!(to_i32(-1e12), i32::MIN);
        assert_eq!(to_i32(12.0), 12);
        assert_eq!(to_u32(-5.0), 0);
        assert_eq!(to_u32(1e12), u32::MAX);
    }
}
