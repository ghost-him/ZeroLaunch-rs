//! Foreground fullscreen detection on macOS.
//!
//! Mirrors the Windows `is_foreground_fullscreen()` contract used by the
//! search-bar wake gate: a normal (layer 0) on-screen window whose frame
//! covers the whole main display counts as fullscreen. Real macOS fullscreen
//! Spaces satisfy this, while a merely maximized window leaves the menu bar
//! and Dock exposed and does not.
//!
//! CGWindowList returns an array of heterogeneous CFDictionary values, so
//! values are read through the raw CoreFoundation C API keyed by CFTypeID —
//! the same approach the `active-win-pos-rs` crate uses on macOS.

use core_foundation::array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef};
use core_foundation::base::{CFGetTypeID, TCFType};
use core_foundation::dictionary::{CFDictionary, CFDictionaryGetValueIfPresent, CFDictionaryRef};
use core_foundation::number::{
    CFNumberGetType, CFNumberGetTypeID, CFNumberGetValue, CFNumberRef, CFNumberType,
};
use core_foundation::string::CFStringRef;
use core_graphics::display::CGDisplay;
use core_graphics::geometry::CGRect;
use core_graphics::window::{
    copy_window_info, kCGNullWindowID, kCGWindowBounds, kCGWindowLayer,
    kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenOnly,
};
use std::ffi::c_void;

/// kCFNumberSInt32Type / kCFNumberSInt64Type values from CFNumberType.
const CF_NUMBER_SINT32: CFNumberType = 3;
const CF_NUMBER_SINT64: CFNumberType = 4;
/// Windows in the normal layer; menus, the Dock and overlays sit above it.
const NORMAL_WINDOW_LAYER: i64 = 0;

/// Reports whether a normal macOS window currently covers the main display.
pub fn is_foreground_fullscreen() -> bool {
    let Some(window_list) = copy_window_info(
        kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements,
        kCGNullWindowID,
    ) else {
        return false;
    };
    let list_ref: CFArrayRef = window_list.as_concrete_TypeRef();
    let count = unsafe { CFArrayGetCount(list_ref) };
    if count <= 0 {
        return false;
    }
    let main_bounds = CGDisplay::main().bounds();
    for index in 0..count {
        let window = unsafe { CFArrayGetValueAtIndex(list_ref, index) };
        if window.is_null() {
            continue;
        }
        // A window dictionary entry; key is CFString, value is heterogeneous.
        let window_dict = window as CFDictionaryRef;
        let Some(layer) = dict_i64(window_dict, unsafe { kCGWindowLayer }) else {
            continue;
        };
        if layer != NORMAL_WINDOW_LAYER {
            continue;
        }
        let Some(bounds) = dict_rect(window_dict, unsafe { kCGWindowBounds }) else {
            continue;
        };
        if window_covers_main(bounds, &main_bounds) {
            return true;
        }
    }
    false
}

/// Reads an integer CFNumber value from a window dict (e.g. kCGWindowLayer).
fn dict_i64(dict: CFDictionaryRef, key: CFStringRef) -> Option<i64> {
    let value = dict_value(dict, key)?;
    if unsafe { CFGetTypeID(value) } != unsafe { CFNumberGetTypeID() } {
        return None;
    }
    let number = value as CFNumberRef;
    let number_type = unsafe { CFNumberGetType(number) };
    match number_type {
        CF_NUMBER_SINT64 => {
            let mut out: i64 = 0;
            let ok =
                unsafe { CFNumberGetValue(number, number_type, (&mut out as *mut i64).cast()) };
            ok.then_some(out)
        }
        CF_NUMBER_SINT32 => {
            let mut out: i32 = 0;
            let ok =
                unsafe { CFNumberGetValue(number, number_type, (&mut out as *mut i32).cast()) };
            ok.then_some(out as i64)
        }
        _ => None,
    }
}

/// Reads the kCGWindowBounds value: a nested CGRect dictionary.
fn dict_rect(dict: CFDictionaryRef, key: CFStringRef) -> Option<CGRect> {
    let value = dict_value(dict, key)?;
    if value.is_null() {
        return None;
    }
    // kCGWindowBounds is itself a CFDictionary of x/y/width/height.
    let bounds_dict = unsafe { CFDictionary::wrap_under_create_rule(value as CFDictionaryRef) };
    CGRect::from_dict_representation(&bounds_dict)
}

/// Looks up a key in a window dictionary and returns the raw value pointer.
fn dict_value(dict: CFDictionaryRef, key: CFStringRef) -> Option<*const c_void> {
    let mut value: *const c_void = std::ptr::null();
    let found = unsafe { CFDictionaryGetValueIfPresent(dict, key.cast::<c_void>(), &mut value) };
    (found != 0 && !value.is_null()).then_some(value)
}

/// Whether a window frame covers the main display frame on all sides.
fn window_covers_main(window: CGRect, main: &CGRect) -> bool {
    window.origin.x <= main.origin.x
        && window.origin.y <= main.origin.y
        && (window.origin.x + window.size.width) >= (main.origin.x + main.size.width)
        && (window.origin.y + window.size.height) >= (main.origin.y + main.size.height)
}
