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
    static FRAME_COUNT: RefCell<u32> = RefCell::new(1);
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

    // Log the characteristic and service UUIDs
    let char_uuid = tx_char.uuid();
    web_sys::console::log_1(&format!("Characteristic UUID: {}", char_uuid).into());
    let svc_uuid = service.uuid();
    web_sys::console::log_1(&format!("Service UUID: {}", svc_uuid).into());

    // Subscribe to notifications
    let on_value_changed = Closure::wrap(Box::new(move |event: Event| {
        let target = event.target().unwrap();
        let constructor = js_sys::Reflect::get(&target, &JsValue::from_str("constructor")).unwrap();
        let name = js_sys::Reflect::get(&constructor, &JsValue::from_str("name")).unwrap();
        // Try to cast the target to BluetoothRemoteGattCharacteristic
        let charac = target.dyn_ref::<BluetoothRemoteGattCharacteristic>();
        if let Some(charac) = charac {
            if let Some(value) = charac.value() {
                // value is a DataView
                let dataview = js_sys::DataView::from(value.clone());
                let buffer = dataview.buffer();
                let byte_offset = dataview.byte_offset();
                let byte_length = dataview.byte_length();
                let arr = js_sys::Uint8Array::new(&buffer).subarray(
                    byte_offset.try_into().unwrap(),
                    (byte_offset + byte_length).try_into().unwrap()
                );
                let data = arr.to_vec();
                web_sys::console::log_1(&format!("Raw data length: {}", data.len()).into());
                if data.len() < 2 {
                    web_sys::console::log_1(&"Received BLE notification with less than 2 bytes".into());
                    return;
                }
                // Print raw data as hex string with frame count
                FRAME_COUNT.with(|count| {
                    let mut c = count.borrow_mut();
                    let hex_str = data.iter().map(|b| format!("0x{:02X}", b)).collect::<Vec<_>>().join(" ");
                    let msg = format!("frame {}: {}", *c, hex_str);
                    web_sys::console::log_1(&msg.into());
                    *c += 1;
                });
                let protocol_id = data[1];
                let mut metrics = Object::new();
                match protocol_id {
                    0x50 => { // State and Speed
                        if data.len() >= 9 {
                            let speed = f32::from_le_bytes([data[4], data[5], data[6], data[7]]);
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
                    0x56 => { // distance
                        if data.len() == 10 {
                            let distance = f32::from_le_bytes([data[6], data[5], data[4], data[3]]);
                            Reflect::set(&metrics, &"distance".into(),&JsValue::from_f64(distance as f64)).ok();
                            web_sys::console::log_1(&format!("distance: {}", distance).into());
                        }
                    },
                    0x57 => { // Step Count
                        if data.len() == 10 {
                            let step_count = u32::from_le_bytes([data[6], data[5], data[4], data[3]]);
                            Reflect::set(&metrics, &"step-count".into(), &JsValue::from(step_count)).ok();
                            web_sys::console::log_1(&format!("Step Count: {}", step_count).into());
                        }
                    },
                    _ => {}
                }
                METRICS_CALLBACK.with(|cb| {
                    if let Some(cb) = &*cb.borrow() {
                        let _ = cb.call1(&JsValue::NULL, &metrics);
                    }
                });
            } else {
                web_sys::console::log_1(&"Characteristic value is None".into());
            }
        } else {
            web_sys::console::log_1(&"Event target is not a BluetoothRemoteGattCharacteristic".into());
        }
    }) as Box<dyn FnMut(_)>);
    tx_char.add_event_listener_with_callback("characteristicvaluechanged", on_value_changed.as_ref().unchecked_ref())?;
    on_value_changed.forget();
    JsFuture::from(tx_char.start_notifications()).await?;

    let value = tx_char.value();
    if let Some(value) = value {
        let dataview = js_sys::DataView::from(value.clone());
        let buffer = dataview.buffer();
        let byte_offset = dataview.byte_offset();
        let byte_length = dataview.byte_length();
        let arr = js_sys::Uint8Array::new(&buffer).subarray(
            byte_offset.try_into().unwrap(),
            (byte_offset + byte_length).try_into().unwrap()
        );
        let data = arr.to_vec();
        web_sys::console::log_1(&format!("Direct read data length: {}", data.len()).into());
    }

    web_sys::console::log_1(&"🔍 BLE UART notifications enabled".into());
    Ok(())
}