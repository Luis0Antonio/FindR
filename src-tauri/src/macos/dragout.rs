use std::path::PathBuf;

use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType, NSView};
use tauri::{AppHandle, Manager};

pub fn start(app: &AppHandle, paths: Vec<String>) {
    let Some(first) = paths.first() else { return };
    let image = super::icons::icon_png(first, 64).unwrap_or_default();
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(window) = handle.get_webview_window("main") else { return };
        let files = paths.into_iter().map(PathBuf::from).collect();
        let options = drag::Options { mode: drag::DragMode::Copy, ..Default::default() };
        let ended = handle.clone();
        let _ = drag::start_drag(&window, drag::DragItem::Files(files), drag::Image::Raw(image), move |_, _| release_mouse(&ended), options);
    });
}

// The native session swallows the mouse-up, so WebKit still thinks the button is held and eats the next click.
// Sent straight to the WKWebView because NSWindow routes it by location, which is outside the window here.
fn release_mouse(app: &AppHandle) {
    let Some(webview) = app.get_webview_window("main") else { return };
    let _ = webview.with_webview(|platform| {
        let view = unsafe { &*(platform.inner() as *const NSView) };
        let Some(window) = view.window() else { return };
        let event = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
            NSEventType::LeftMouseUp,
            window.mouseLocationOutsideOfEventStream(),
            NSEventModifierFlags::empty(),
            0.0,
            window.windowNumber(),
            None,
            0,
            1,
            0.0,
        );
        if let Some(event) = event {
            view.mouseUp(&event);
        }
    });
}
