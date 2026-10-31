use std::sync::mpsc;
use std::time::Duration;

use block2::RcBlock;
use objc2::rc::{autoreleasepool, Retained};
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{msg_send, AnyThread};
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSImage, NSWorkspace};
use objc2_foundation::{NSDictionary, NSPoint, NSRect, NSSize, NSString, NSURL};

#[link(name = "QuickLookThumbnailing", kind = "framework")]
extern "C" {}

const QL_REPRESENTATION_THUMBNAIL: usize = 1 << 2;

pub fn icon_png(path: &str, px: u32) -> Option<Vec<u8>> {
    autoreleasepool(|_| {
        let image = NSWorkspace::sharedWorkspace().iconForFile(&NSString::from_str(path));
        let size = NSSize::new(px as f64, px as f64);
        image.setSize(size);
        image_png(&image, size)
    })
}

/// Content preview from Quick Look, fitted inside a `px` square. `None` if Quick Look has no thumbnail for the file.
pub fn thumbnail_png(path: &str, px: u32) -> Option<Vec<u8>> {
    let (tx, rx) = mpsc::channel();
    autoreleasepool(|_| {
        let url = NSURL::fileURLWithPath(&NSString::from_str(path));
        let size = NSSize::new(px as f64, px as f64);
        let request_class = AnyClass::get(c"QLThumbnailGenerationRequest")?;
        let generator_class = AnyClass::get(c"QLThumbnailGenerator")?;
        unsafe {
            let request: Retained<AnyObject> = msg_send![
                msg_send![request_class, alloc],
                initWithFileAtURL: &*url,
                size: size,
                scale: 1.0f64,
                representationTypes: QL_REPRESENTATION_THUMBNAIL
            ];
            let generator: Retained<AnyObject> = msg_send![generator_class, sharedGenerator];
            let done = RcBlock::new(move |rep: *mut AnyObject, _error: *mut AnyObject| {
                let png = rep.as_ref().and_then(|rep| {
                    let image: Option<Retained<NSImage>> = msg_send![rep, NSImage];
                    autoreleasepool(|_| image.and_then(|image| image_png(&image, image.size())))
                });
                let _ = tx.send(png);
            });
            let _: () = msg_send![&generator, generateBestRepresentationForRequest: &*request, completionHandler: &*done];
        }
        Some(())
    })?;
    rx.recv_timeout(Duration::from_secs(5)).ok().flatten()
}

fn image_png(image: &NSImage, size: NSSize) -> Option<Vec<u8>> {
    let mut rect = NSRect::new(NSPoint::new(0.0, 0.0), size);
    let cg = unsafe { image.CGImageForProposedRect_context_hints(&mut rect, None, None) }?;
    let rep = NSBitmapImageRep::initWithCGImage(NSBitmapImageRep::alloc(), &cg);
    let data = unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new()) }?;
    Some(data.to_vec())
}

pub fn is_file_package(path: &str) -> bool {
    autoreleasepool(|_| NSWorkspace::sharedWorkspace().isFilePackageAtPath(&NSString::from_str(path)))
}

/// Finder's kHasCustomIcon bit lives in the FinderInfo xattr.
pub fn has_custom_icon(path: &str) -> bool {
    let Ok(cpath) = std::ffi::CString::new(path) else { return false };
    let mut buf = [0u8; 32];
    let n = unsafe {
        libc::getxattr(cpath.as_ptr(), c"com.apple.FinderInfo".as_ptr(), buf.as_mut_ptr().cast(), buf.len(), 0, 0)
    };
    n >= 9 && buf[8] & 0x04 != 0
}
