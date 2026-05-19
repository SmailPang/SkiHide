use std::collections::BTreeSet;

use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::Graphics::Gdi::{
    EnumFontFamiliesExW, GetDC, ReleaseDC, FONTENUMPROCW, LOGFONTW, TEXTMETRICW,
    DEFAULT_CHARSET,
};

struct EnumContext {
    families: BTreeSet<String>,
}

pub fn list_system_font_families() -> Vec<String> {
    let mut context = EnumContext {
        families: BTreeSet::new(),
    };

    unsafe {
        let hdc = GetDC(Some(HWND::default()));
        if hdc.0.is_null() {
            return Vec::new();
        }

        let mut logfont = LOGFONTW::default();
        logfont.lfCharSet = DEFAULT_CHARSET;

        let proc: FONTENUMPROCW = Some(enum_font_families);
        let lparam = LPARAM(&mut context as *mut EnumContext as isize);
        let _ = EnumFontFamiliesExW(hdc, &logfont, proc, lparam, 0);
        let _ = ReleaseDC(Some(HWND::default()), hdc);
    }

    context.families.into_iter().collect()
}

unsafe extern "system" fn enum_font_families(
    logfont: *const LOGFONTW,
    _text_metric: *const TEXTMETRICW,
    _font_type: u32,
    lparam: LPARAM,
) -> i32 {
    if logfont.is_null() {
        return 1;
    }

    let context = &mut *(lparam.0 as *mut EnumContext);
    let face_name = &(*logfont).lfFaceName;
    let end = face_name
        .iter()
        .position(|&ch| ch == 0)
        .unwrap_or(face_name.len());
    let name = String::from_utf16_lossy(&face_name[..end]);
    let name = name.trim();

    if name.is_empty() || name.starts_with('@') {
        return 1;
    }

    context.families.insert(name.to_string());
    1
}
