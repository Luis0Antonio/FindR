use objc2::rc::autoreleasepool;
use objc2::AnyThread;
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSWorkspace};
use objc2_foundation::{NSDictionary, NSPoint, NSRect, NSSize, NSString};

pub fn icon_png(path: &str, px: u32) -> Option<Vec<u8>> {
    autoreleasepool(|_| {
        let image = NSWorkspace::sharedWorkspace().iconForFile(&NSString::from_str(path));
        let size = NSSize::new(px as f64, px as f64);
        image.setSize(size);
        let mut rect = NSRect::new(NSPoint::new(0.0, 0.0), size);
        let cg = unsafe { image.CGImageForProposedRect_context_hints(&mut rect, None, None) }?;
        let rep = NSBitmapImageRep::initWithCGImage(NSBitmapImageRep::alloc(), &cg);
        let data = unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new()) }?;
        Some(data.to_vec())
    })
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
