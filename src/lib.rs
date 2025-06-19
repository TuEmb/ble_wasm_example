use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, RequestDeviceOptions};

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub async fn scan_ble_devices() -> Result<(), JsValue> {
    let navigator = window().unwrap().navigator();
    let bluetooth = navigator.bluetooth().ok_or_else(|| JsValue::from_str("Bluetooth API not support for this browser"))?;

    let mut options = RequestDeviceOptions::new();
    options.set_accept_all_devices(true);

    let device_promise = bluetooth.request_device(&options);
    let device = JsFuture::from(device_promise).await?;

    web_sys::console::log_1(&"🔍 Found BLE device:".into());
    web_sys::console::log_1(&device);

    Ok(())
}
