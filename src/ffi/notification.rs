//! Diagnostic notification callback support for the C ABI.

use std::ffi::{c_char, c_void, CString};

use crate::ffi::error::{clear_last_error, set_last_error};
use crate::ffi::object::FfiDocument;
use crate::ffi::types::{DwgData, DWG_EINVAL, DWG_OK};
use crate::notification::NotificationType;

/// Callback invoked synchronously for a diagnostic produced by a read.
///
/// The message pointer is valid only during the callback. `user_data` is the
/// opaque value supplied to `dwg_set_notification_callback`.
pub type DwgNotificationCallback =
    unsafe extern "C" fn(notification_type: i32, message: *const c_char, user_data: *mut c_void);

/// Notification code for an unsupported implementation.
pub const DWG_NOTIFICATION_NOT_IMPLEMENTED: i32 = 1;
/// Notification code for a feature unsupported in the current context.
pub const DWG_NOTIFICATION_NOT_SUPPORTED: i32 = 2;
/// Notification code for a recoverable warning.
pub const DWG_NOTIFICATION_WARNING: i32 = 3;
/// Notification code for a recoverable error.
pub const DWG_NOTIFICATION_ERROR: i32 = 4;

fn notification_code(notification_type: NotificationType) -> i32 {
    match notification_type {
        NotificationType::NotImplemented => DWG_NOTIFICATION_NOT_IMPLEMENTED,
        NotificationType::NotSupported => DWG_NOTIFICATION_NOT_SUPPORTED,
        NotificationType::Warning => DWG_NOTIFICATION_WARNING,
        NotificationType::Error => DWG_NOTIFICATION_ERROR,
    }
}

impl FfiDocument {
    pub(crate) fn dispatch_notifications(&self) {
        let Some(callback) = self.notification_callback else {
            return;
        };
        let user_data = self.notification_user_data;
        let messages = self
            .doc
            .notifications
            .iter()
            .map(|notification| {
                let message = notification.message.replace('\0', " ");
                (
                    notification_code(notification.notification_type),
                    CString::new(message).expect("NUL characters were sanitized"),
                )
            })
            .collect::<Vec<_>>();

        for (code, message) in messages {
            unsafe { callback(code, message.as_ptr(), user_data) };
        }
    }
}

/// Install or clear the diagnostic callback for a document.
///
/// A callback is invoked synchronously after a successful read that produced
/// notifications. Passing `NULL` clears the callback and user data.
#[no_mangle]
pub unsafe extern "C" fn dwg_set_notification_callback(
    data: *mut DwgData,
    callback: Option<DwgNotificationCallback>,
    user_data: *mut c_void,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    let ffi = &mut *(data as *mut FfiDocument);
    ffi.notification_callback = callback;
    ffi.notification_user_data = if callback.is_some() {
        user_data
    } else {
        std::ptr::null_mut()
    };
    clear_last_error();
    DWG_OK
}
