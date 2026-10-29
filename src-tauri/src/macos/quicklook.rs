use std::cell::{Cell, OnceCell, RefCell};
use std::sync::OnceLock;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{define_class, msg_send, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSEvent, NSEventType, NSResponder, NSWindow, NSWindowDelegate};
use objc2_foundation::{NSInteger, NSObjectProtocol, NSString, NSURL};
use objc2_quick_look_ui::{QLPreviewItem, QLPreviewPanel, QLPreviewPanelDataSource, QLPreviewPanelDelegate};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

const KEY_SPACE: u16 = 49;
const KEY_DOWN: u16 = 125;
const KEY_UP: u16 = 126;

static APP: OnceLock<AppHandle> = OnceLock::new();

thread_local! {
    // QLPreviewPanel holds dataSource/delegate weakly, so the controller must live for the whole app.
    static CONTROLLER: OnceCell<Retained<QlController>> = const { OnceCell::new() };
}

pub struct Ivars {
    paths: RefCell<Vec<String>>,
    index: Cell<usize>,
}

define_class!(
    #[unsafe(super(NSResponder))]
    #[thread_kind = MainThreadOnly]
    #[name = "RFQuickLookController"]
    #[ivars = Ivars]
    pub struct QlController;

    impl QlController {
        #[unsafe(method(acceptsPreviewPanelControl:))]
        fn accepts_control(&self, _panel: Option<&QLPreviewPanel>) -> bool {
            true
        }

        #[unsafe(method(beginPreviewPanelControl:))]
        fn begin_control(&self, panel: Option<&QLPreviewPanel>) {
            let Some(panel) = panel else { return };
            unsafe {
                panel.setDataSource(Some(ProtocolObject::from_ref(self)));
                panel.setDelegate(Some(self.as_any()));
                panel.reloadData();
                panel.setCurrentPreviewItemIndex(self.ivars().index.get() as NSInteger);
            }
        }

        #[unsafe(method(endPreviewPanelControl:))]
        fn end_control(&self, panel: Option<&QLPreviewPanel>) {
            if let Some(panel) = panel {
                unsafe {
                    panel.setDataSource(None);
                    panel.setDelegate(None);
                }
            }
            emit("quicklook://closed", json!({}));
        }
    }

    unsafe impl NSObjectProtocol for QlController {}

    unsafe impl NSWindowDelegate for QlController {}

    unsafe impl QLPreviewPanelDataSource for QlController {
        #[unsafe(method(numberOfPreviewItemsInPreviewPanel:))]
        fn number_of_items(&self, _panel: Option<&QLPreviewPanel>) -> NSInteger {
            self.ivars().paths.borrow().len() as NSInteger
        }

        #[unsafe(method_id(previewPanel:previewItemAtIndex:))]
        fn item_at(
            &self,
            _panel: Option<&QLPreviewPanel>,
            index: NSInteger,
        ) -> Option<Retained<ProtocolObject<dyn QLPreviewItem>>> {
            self.item(index)
        }
    }

    unsafe impl QLPreviewPanelDelegate for QlController {
        #[unsafe(method(previewPanel:handleEvent:))]
        fn handle_event(&self, panel: Option<&QLPreviewPanel>, event: Option<&NSEvent>) -> bool {
            handle_key(panel, event)
        }
    }
);

impl QlController {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Ivars { paths: RefCell::new(Vec::new()), index: Cell::new(0) });
        unsafe { msg_send![super(this), init] }
    }

    fn item(&self, index: NSInteger) -> Option<Retained<ProtocolObject<dyn QLPreviewItem>>> {
        let paths = self.ivars().paths.borrow();
        let path = paths.get(usize::try_from(index).ok()?)?;
        let url = NSURL::fileURLWithPath(&NSString::from_str(path));
        Some(ProtocolObject::from_retained(url))
    }

    fn as_any(&self) -> &AnyObject {
        let obj: &NSResponder = self;
        let obj: &AnyObject = obj.as_ref();
        obj
    }
}

fn handle_key(panel: Option<&QLPreviewPanel>, event: Option<&NSEvent>) -> bool {
    let Some(event) = event else { return false };
    if event.r#type() != NSEventType::KeyDown {
        return false;
    }
    match event.keyCode() {
        KEY_UP => emit("quicklook://navigate", json!({ "delta": -1 })),
        KEY_DOWN => emit("quicklook://navigate", json!({ "delta": 1 })),
        KEY_SPACE => {
            if let Some(panel) = panel {
                panel.orderOut(None);
            }
        }
        _ => return false,
    }
    true
}

fn emit(event: &str, payload: serde_json::Value) {
    if let Some(app) = APP.get() {
        let _ = app.emit(event, payload);
    }
}

fn controller(mtm: MainThreadMarker, window: &NSWindow) -> Retained<QlController> {
    CONTROLLER.with(|cell| {
        cell.get_or_init(|| {
            let ctrl = QlController::new(mtm);
            unsafe {
                ctrl.setNextResponder(window.nextResponder().as_deref());
                window.setNextResponder(Some(&ctrl));
            }
            ctrl
        })
        .clone()
    })
}

fn ns_window(app: &AppHandle) -> Option<Retained<NSWindow>> {
    let window = app.get_webview_window("main")?;
    let ptr = window.ns_window().ok()? as *mut NSWindow;
    unsafe { Retained::retain(ptr) }
}

fn set_items(ctrl: &QlController, paths: Vec<String>, index: usize) {
    let index = index.min(paths.len().saturating_sub(1));
    *ctrl.ivars().paths.borrow_mut() = paths;
    ctrl.ivars().index.set(index);
}

pub fn init(app: &AppHandle) {
    let _ = APP.set(app.clone());
}

pub fn show(app: &AppHandle, paths: Vec<String>, index: usize) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let mtm = MainThreadMarker::new().expect("main thread");
        let Some(window) = ns_window(&handle) else { return };
        let ctrl = controller(mtm, &window);
        set_items(&ctrl, paths, index);
        let panel = unsafe { QLPreviewPanel::sharedPreviewPanel(mtm) };
        let Some(panel) = panel else { return };
        if panel.isVisible() {
            unsafe {
                panel.reloadData();
                panel.setCurrentPreviewItemIndex(ctrl.ivars().index.get() as NSInteger);
            }
        } else {
            panel.makeKeyAndOrderFront(None);
        }
    });
}

pub fn update(app: &AppHandle, paths: Vec<String>, index: usize) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let mtm = MainThreadMarker::new().expect("main thread");
        if !unsafe { QLPreviewPanel::sharedPreviewPanelExists(mtm) } {
            return;
        }
        let Some(window) = ns_window(&handle) else { return };
        let ctrl = controller(mtm, &window);
        set_items(&ctrl, paths, index);
        let Some(panel) = (unsafe { QLPreviewPanel::sharedPreviewPanel(mtm) }) else { return };
        if panel.isVisible() {
            unsafe {
                panel.reloadData();
                panel.setCurrentPreviewItemIndex(ctrl.ivars().index.get() as NSInteger);
            }
        }
    });
}

pub fn close(app: &AppHandle) {
    let _ = app.run_on_main_thread(move || {
        let mtm = MainThreadMarker::new().expect("main thread");
        if !unsafe { QLPreviewPanel::sharedPreviewPanelExists(mtm) } {
            return;
        }
        if let Some(panel) = unsafe { QLPreviewPanel::sharedPreviewPanel(mtm) } {
            if panel.isVisible() {
                panel.orderOut(None);
            }
        }
    });
}
