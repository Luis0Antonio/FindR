use objc2::rc::autoreleasepool;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeFileURL, NSPasteboardWriting};
use objc2_foundation::{NSArray, NSString, NSURL};

/// Puts the files on the general pasteboard, as Finder does, and returns its change count.
pub fn write_files(paths: &[String]) -> isize {
    autoreleasepool(|_| {
        let pasteboard = NSPasteboard::generalPasteboard();
        pasteboard.clearContents();
        let urls: Vec<_> = paths.iter().map(|p| NSURL::fileURLWithPath(&NSString::from_str(p))).collect();
        let objects: Vec<&ProtocolObject<dyn NSPasteboardWriting>> = urls.iter().map(|u| ProtocolObject::from_ref(&**u)).collect();
        pasteboard.writeObjects(&NSArray::from_slice(&objects));
        pasteboard.changeCount()
    })
}

/// File paths on the general pasteboard (empty if it holds no files), with its change count.
pub fn read_files() -> (Vec<String>, isize) {
    autoreleasepool(|_| {
        let pasteboard = NSPasteboard::generalPasteboard();
        let paths = pasteboard
            .pasteboardItems()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.stringForType(unsafe { NSPasteboardTypeFileURL }))
                    .filter_map(|s| NSURL::URLWithString(&s))
                    .filter_map(|url| url.path())
                    .map(|p| p.to_string())
                    .collect()
            })
            .unwrap_or_default();
        (paths, pasteboard.changeCount())
    })
}
