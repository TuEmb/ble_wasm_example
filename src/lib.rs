use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, RequestDeviceOptions, BluetoothRemoteGattService, BluetoothRemoteGattCharacteristic, BluetoothRemoteGattServer, BluetoothDevice, Event};
use js_sys::{Function, Reflect, Object, Array, Uint8Array};
use std::cell::RefCell;
use std::rc::Rc;

// Nordic UART UUIDs
const UART_SERVICE_UUID: &str = "6e400001-b5a3-f393-e0a9-e50e24dcca9e";
const UART_TX_CHAR_UUID: &str = "6e400003-b5a3-f393-e0a9-e50e24dcca9e";

thread_local! {
    static METRICS_CALLBACK: RefCell<Option<Function>> = RefCell::new(None);
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn set_metrics_callback(cb: &Function) {
    METRICS_CALLBACK.with(|f| *f.borrow_mut() = Some(cb.clone()));
}

#[wasm_bindgen]
pub async fn scan_ble_devices() -> Result<(), JsValue> {
    let navigator = window().unwrap().navigator();
    let bluetooth = navigator.bluetooth().ok_or_else(|| JsValue::from_str("Bluetooth API not support for this browser"))?;

    let mut options = RequestDeviceOptions::new();
    options.set_accept_all_devices(true);
    options.set_optional_services(&Array::of1(&JsValue::from_str(UART_SERVICE_UUID)));

    let device_promise = bluetooth.request_device(&options);
    let device = JsFuture::from(device_promise).await?;
    let device: BluetoothDevice = device.dyn_into()?;
    let gatt = device.gatt().ok_or_else(|| JsValue::from_str("No GATT"))?;
    let server = JsFuture::from(gatt.connect()).await?;
    let server = server.dyn_into::<BluetoothRemoteGattServer>()?;
    let service = JsFuture::from(server.get_primary_service_with_str(UART_SERVICE_UUID)).await?;
    let service: BluetoothRemoteGattService = service.dyn_into()?;
    let tx_char = JsFuture::from(service.get_characteristic_with_str(UART_TX_CHAR_UUID)).await?;
    let tx_char: BluetoothRemoteGattCharacteristic = tx_char.dyn_into()?;

    // Subscribe to notifications
    let on_value_changed = Closure::wrap(Box::new(move |event: Event| {
        let charac = event.target().unwrap().dyn_into::<BluetoothRemoteGattCharacteristic>().unwrap();
        let value = charac.value().unwrap();
        let arr = Uint8Array::new(&value);
        let data = arr.to_vec();
        if data.len() < 2 { return; }
        let protocol_id = data[1];
        let mut metrics = Object::new();
        match protocol_id {
            0x50 => { // State and Speed
                if data.len() >= 9 {
                    let state = data[3];
                    let speed = f32::from_le_bytes([data[4], data[5], data[6], data[7]]);
                    Reflect::set(&metrics, &"state".into(), &JsValue::from(state)).ok();
                    Reflect::set(&metrics, &"speed".into(), &JsValue::from_f64(speed as f64)).ok();
                }
            },
            0x51 => { // Step Event
                if data.len() >= 13 {
                    let step_length = f32::from_le_bytes([data[3], data[4], data[5], data[6]]);
                    let stride_length = f32::from_le_bytes([data[7], data[8], data[9], data[10]]);
                    Reflect::set(&metrics, &"step_length".into(), &JsValue::from_f64(step_length as f64)).ok();
                    Reflect::set(&metrics, &"stride_length".into(), &JsValue::from_f64(stride_length as f64)).ok();
                }
            },
            _ => {}
        }
        METRICS_CALLBACK.with(|cb| {
            if let Some(cb) = &*cb.borrow() {
                let _ = cb.call1(&JsValue::NULL, &metrics);
            }
        });
    }) as Box<dyn FnMut(_)>);
    tx_char.set_oncharacteristicvaluechanged(Some(on_value_changed.as_ref().unchecked_ref()));
    on_value_changed.forget();
    JsFuture::from(tx_char.start_notifications()).await?;

    web_sys::console::log_1(&"🔍 BLE UART notifications enabled".into());
    Ok(())
}
