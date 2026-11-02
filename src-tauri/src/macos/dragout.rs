use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{Bool, NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{define_class, msg_send, AllocAnyThread, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSAffineTransformNSAppKitAdditions, NSBezierPath, NSColor, NSCompositingOperation, NSDragOperation, NSDraggingContext, NSDraggingItem,
    NSDraggingSession, NSDraggingSource, NSEvent, NSEventModifierFlags, NSEventType, NSGraphicsContext, NSImage, NSImageInterpolation,
    NSShadow, NSView, NSWindow,
};
use objc2_foundation::{NSAffineTransform, NSArray, NSData, NSPoint, NSRect, NSSize, NSString, NSURL};
use tauri::{AppHandle, Manager};

/// Pixel size of each preview, enough for `CARD` points on Retina.
const PREVIEW_PX: u32 = 192;
const CARD: f64 = 96.0;
const CANVAS: f64 = 140.0;
/// Counterclockwise tilt of the front card and the ones fanned out behind it; the UI's drag ghost mirrors these.
const TILTS: [f64; 3] = [6.0, -5.0, 14.0];

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "FindRDragSource"]
    #[ivars = AppHandle]
    struct DragSource;

    unsafe impl NSObjectProtocol for DragSource {}

    unsafe impl NSDraggingSource for DragSource {
        #[unsafe(method(draggingSession:sourceOperationMaskForDraggingContext:))]
        fn operation_mask(&self, _session: &NSDraggingSession, _context: NSDraggingContext) -> NSDragOperation {
            NSDragOperation::Copy
        }

        #[unsafe(method(draggingSession:endedAtPoint:operation:))]
        fn ended(&self, _session: &NSDraggingSession, _point: NSPoint, _operation: NSDragOperation) {
            release_mouse(self.ivars());
        }
    }
);

impl DragSource {
    fn new(app: AppHandle, mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(app);
        unsafe { msg_send![super(this), init] }
    }
}

pub fn start(app: &AppHandle, paths: Vec<String>) {
    if paths.is_empty() {
        return;
    }
    let previews: Vec<_> = paths.iter().take(TILTS.len()).filter_map(|p| crate::icon_protocol::preview_png(p, PREVIEW_PX)).collect();
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || begin(&handle, &paths, previews));
}

fn begin(app: &AppHandle, paths: &[String], previews: Vec<(Vec<u8>, bool)>) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let Some(window) = app.get_webview_window("main") else { return };
    let Ok(ns_window) = window.ns_window() else { return };
    let ns_window = unsafe { &*(ns_window as *const NSWindow) };
    let Some(content_view) = ns_window.contentView() else { return };

    let image = drag_image(previews);
    let at = ns_window.mouseLocationOutsideOfEventStream();
    let frame = NSRect::new(NSPoint::new(at.x - CANVAS / 2.0, at.y - CANVAS / 2.0), NSSize::new(CANVAS, CANVAS));
    // One item per file so every app gets them all, but only the first draws the picture; macOS adds the count badge.
    let items: Vec<_> = paths
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let url = NSURL::fileURLWithPath(&NSString::from_str(path));
            let item = NSDraggingItem::initWithPasteboardWriter(NSDraggingItem::alloc(), &ProtocolObject::from_retained(url));
            unsafe { item.setDraggingFrame_contents(frame, if i == 0 { Some(&*image) } else { None }) };
            item
        })
        .collect();
    let event = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
        NSEventType::LeftMouseDragged,
        at,
        NSEventModifierFlags::empty(),
        objc2_app_kit::NSApp(mtm).currentEvent().map(|e| e.timestamp()).unwrap_or(0.0),
        ns_window.windowNumber(),
        None,
        0,
        1,
        1.0,
    );
    let Some(event) = event else { return };
    let source = DragSource::new(app.clone(), mtm);
    content_view.beginDraggingSessionWithItems_event_source(&NSArray::from_retained_slice(&items), &event, ProtocolObject::from_ref(&*source));
}

/// Rounded, tilted, translucent cards for up to three files, the first one on top.
fn drag_image(previews: Vec<(Vec<u8>, bool)>) -> Retained<NSImage> {
    let cards: Vec<_> = previews
        .into_iter()
        .filter_map(|(png, thumbnail)| NSImage::initWithData(NSImage::alloc(), &NSData::from_vec(png)).map(|img| card(img, thumbnail)))
        .collect();
    let draw = RcBlock::new(move |_: NSRect| -> Bool {
        if let Some(context) = NSGraphicsContext::currentContext() {
            context.setImageInterpolation(NSImageInterpolation::High);
        }
        for (card, tilt) in cards.iter().zip(TILTS).rev() {
            NSGraphicsContext::saveGraphicsState_class();
            let transform = NSAffineTransform::transform();
            transform.translateXBy_yBy(CANVAS / 2.0, CANVAS / 2.0);
            transform.rotateByDegrees(tilt);
            transform.concat();
            let shadow = NSShadow::new();
            shadow.setShadowColor(Some(&NSColor::colorWithWhite_alpha(0.0, 0.35)));
            shadow.setShadowBlurRadius(10.0);
            shadow.setShadowOffset(NSSize::new(0.0, -3.0));
            shadow.set();
            let size = card.size();
            let rect = NSRect::new(NSPoint::new(-size.width / 2.0, -size.height / 2.0), size);
            card.drawInRect_fromRect_operation_fraction(rect, NSRect::ZERO, NSCompositingOperation::SourceOver, 0.85);
            NSGraphicsContext::restoreGraphicsState_class();
        }
        Bool::YES
    });
    NSImage::imageWithSize_flipped_drawingHandler(NSSize::new(CANVAS, CANVAS), false, &draw)
}

/// A content thumbnail becomes a photo-like card with rounded corners and a light edge; an icon keeps its own shape.
fn card(image: Retained<NSImage>, thumbnail: bool) -> Retained<NSImage> {
    let size = image.size();
    let scale = CARD / size.width.max(size.height).max(1.0);
    let size = NSSize::new(size.width * scale, size.height * scale);
    let draw = RcBlock::new(move |rect: NSRect| -> Bool {
        if !thumbnail {
            image.drawInRect(rect);
            return Bool::YES;
        }
        let shape = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(rect, 9.0, 9.0);
        NSGraphicsContext::saveGraphicsState_class();
        shape.addClip();
        image.drawInRect(rect);
        NSGraphicsContext::restoreGraphicsState_class();
        let edge = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(NSRect::new(NSPoint::new(rect.origin.x + 1.0, rect.origin.y + 1.0), NSSize::new(rect.size.width - 2.0, rect.size.height - 2.0)), 8.0, 8.0);
        edge.setLineWidth(2.0);
        NSColor::colorWithWhite_alpha(1.0, 0.7).setStroke();
        edge.stroke();
        Bool::YES
    });
    NSImage::imageWithSize_flipped_drawingHandler(size, false, &draw)
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
