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
pub struct WebGame {
    canvas_id: u32,
}

#[wasm_bindgen]
impl WebGame {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WebGame {
        wasm_logger::init(wasm_logger::Config::default());
        console_error_panic_hook::set_once();

        WebGame { canvas_id: 0 }
    }

    pub fn create_render(&mut self, element: String) -> Promise {
        self.canvas_id += 1;
        let id = self.canvas_id;
        log::info!("creating render: {}:{}", element, id);
        future_to_promise(async move { WebGameRender::new(&element, id).await.map(JsValue::from) })
    }
}
