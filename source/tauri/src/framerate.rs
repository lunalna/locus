// WebKit limits pages to about 60 frames per second by default, also on 120 Hz displays.
// The limit is only kept when the performance settings ask for 60 fps or less.

use objc2::msg_send;
use objc2::runtime::{AnyClass, AnyObject, Bool};
use objc2_foundation::NSString;
use tauri::Webview;

pub fn set_limit(webview: &Webview, limit: bool) {
    let _ = webview.with_webview(move |platform| unsafe {
        let view: &AnyObject = &*platform.inner().cast();
        let config: &AnyObject = msg_send![view, configuration];
        let preferences: &AnyObject = msg_send![config, preferences];
        let Some(class) = AnyClass::get(c"WKPreferences") else {
            return;
        };
        let features: &AnyObject = msg_send![class, _features];
        let count: usize = msg_send![features, count];
        for i in 0..count {
            let feature: &AnyObject = msg_send![features, objectAtIndex: i];
            let key: &NSString = msg_send![feature, key];
            if key.to_string() == "PreferPageRenderingUpdatesNear60FPSEnabled" {
                let _: () = msg_send![preferences, _setEnabled: Bool::new(limit), forFeature: feature];
            }
        }
    });
}
