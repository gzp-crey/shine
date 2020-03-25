use raw_window_handle::{web::WebHandle, HasRawWindowHandle, RawWindowHandle};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::HtmlCanvasElement;

pub struct WebWindow {
    canvas: HtmlCanvasElement,
    id: u32,
}

impl WebWindow {
    pub fn new(canvas: HtmlCanvasElement, id: u32) -> WebWindow {
        canvas.set_attribute("data-raw-handle", &id.to_string());
        WebWindow { canvas, id }
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
        Ok(WebWindow::new(canvas, id))
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
