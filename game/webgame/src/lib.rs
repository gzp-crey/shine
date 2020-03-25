#![cfg(target_arch = "wasm32")]

use console_error_panic_hook;
use js_sys::Promise;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::future_to_promise;
use wasm_bindgen_macro::wasm_bindgen;
use wasm_logger;

mod inputmapper;
mod webgamerender;
mod webwindow;

use webgamerender::WebGameRender;

#[wasm_bindgen]
pub struct WebGame {}

#[wasm_bindgen]
impl WebGame {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WebGame {
        wasm_logger::init(wasm_logger::Config::default());
        console_error_panic_hook::set_once();

        WebGame {}
    }

    pub fn create_render(&self, element: String) -> Promise {
        log::info!("createing render: {}", element);
        future_to_promise(async move { WebGameRender::new(&element).await.map(JsValue::from) })
    }
}
