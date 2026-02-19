use std::ffi::{CStr, CString, c_char};

use super::Error;

pub(super) fn c_string_to_string(value: *mut c_char) -> String {
    if value.is_null() {
        return String::new();
    }

    unsafe { CStr::from_ptr(value) }
        .to_string_lossy()
        .into_owned()
}

pub(super) fn to_optional_cstring(value: &Option<String>) -> Result<Option<CString>, Error> {
    value
        .as_ref()
        .map(|text| CString::new(text.as_str()).map_err(|_| Error::InvalidInput))
        .transpose()
}
