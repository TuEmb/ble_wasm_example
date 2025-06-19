# Web Bluetooth Rust Example

This project demonstrates using Rust and WebAssembly to scan for Bluetooth Low Energy (BLE) devices in the browser.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- [npm](https://nodejs.org/) (for running a local web server)

## Build

1. Build the WebAssembly package using `wasm-pack`:
   
   **Windows**

    ```sh
    set RUSTFLAGS=--cfg=web_sys_unstable_apis
    wasm-pack build --target web
    ```

   **Linux/MacOS**
    ```sh
    export RUSTFLAGS="--cfg=web_sys_unstable_apis"
    wasm-pack build --target web
    ```

    This will generate the `pkg/` directory with the necessary JS and WASM files.

## Run

2. Serve the project locally (you need a web server to use Web Bluetooth):

    ```sh
    npx serve .
    ```

    Or use any other static file server (e.g., `python -m http.server`).

3. Open [http://localhost:5000](http://localhost:5000) (or the port shown in your terminal) in a supported browser (e.g., Chrome).

4. Click the **Scan BLE Devices** button to start scanning for BLE devices.

## Notes

- Web Bluetooth only works on secure contexts (localhost or HTTPS).
- Make sure your browser supports the [Web Bluetooth API](https://developer.mozilla.org/en-US/docs/Web/API/Web_Bluetooth_API).

## Project Structure

- [`src/lib.rs`](src/lib.rs): Rust source code for BLE scanning.
- [`index.html`](index.html): Web page UI and JS glue code.
- [`pkg/`](pkg/): Generated WebAssembly and JS bindings.
