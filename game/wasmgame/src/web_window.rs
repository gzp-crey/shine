use js_sys;
use raw_window_handle::{web::WebHandle, HasRawWindowHandle, RawWindowHandle};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{self, HtmlCanvasElement};

pub struct WebWindow {
    canvas: HtmlCanvasElement,
    id: u32,
}

impl WebWindow {
    pub fn new(canvas: HtmlCanvasElement, id: u32) -> Result<WebWindow, JsValue> {
        canvas.set_attribute("data-raw-handle", &id.to_string())?;
        Ok(WebWindow { canvas, id })
    }

    pub fn from_element_by_id(element: &str, id: u32) -> Result<WebWindow, JsValue> {
        let window = web_sys::window().ok_or_else(|| js_sys::Error::new("web window not found"))?;
        let document = window
            .document()
            .ok_or_else(|| js_sys::Error::new("web documnet not found"))?;
        let canvas = document
            .get_element_by_id(element)
            .ok_or_else(|| js_sys::Error::new(&format!("html element [{}] not found", element)))?
            .dyn_into::<HtmlCanvasElement>()?;
        WebWindow::new(canvas, id)
    }

    pub fn inner_size(&self) -> (u32, u32) {
        (self.canvas.width(), self.canvas.height())
    }

    fn raw_window_handle(&self) -> RawWindowHandle {
        let handle = WebHandle {
            id: self.id,
            ..WebHandle::empty()
        };

        RawWindowHandle::Web(handle)
    }
}

unsafe impl HasRawWindowHandle for WebWindow {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.raw_window_handle()
    }
}
