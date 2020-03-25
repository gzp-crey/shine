use crate::inputmapper::{WebInputEvent, WebInputMapper};
use crate::webwindow::WebWindow;
use js_sys;
use shine_game::{render::GameRender, wgpu};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_macro::wasm_bindgen;
use web_sys::{HtmlCanvasElement, WebGlRenderingContext};

struct Inner {
    window: WebWindow,
    render: GameRender,
    input_mapper: WebInputMapper,
}

#[wasm_bindgen]
pub struct WebGameRender {
    inner: Rc<RefCell<Inner>>,
}

impl WebGameRender {
    pub async fn new(element: &str) -> Result<WebGameRender, JsValue> {
        let window = WebWindow::from_element_by_id(element)?;
        //window.attach_mouse_down_handler()?;

        let surface = wgpu::Surface::create(&window);
        let render = GameRender::new(surface)
            .await
            .map_err(|err| js_sys::Error::new(&format!("{:?}", err)))?;

        let inner = Rc::new(RefCell::new(Inner {
            window,
            render,
            input_mapper: WebInputMapper::new(),
        }));

        Ok(WebGameRender { inner })
    }

    /*fn attach_mouse_down_handler(&mut self) -> Result<(), JsValue> {
        let inner = self.inner.clone();
        let handler = move |event: web_sys::MouseEvent| {
            let Inner {
                ref mut render,
                ref input_mapper,
                ref window,
            } = &mut *inner.borrow_mut();

            let w = canvas.width() as f32;
            let h = canvas.height() as f32;
            let x = event.client_x() as f32;
            let y = event.client_y() as f32;
            // experiment shoves that (x,y) is in the [0..w]x[0..h] range
            let x = if w <= 0. { 0. } else { x / w };
            let y = if y <= 0. { 0. } else { y / h };
            render.input.handle_input(input_mapper, &WebInputEvent::MouseDown(x, y));
        };

        let handler = Closure::wrap(Box::new(handler) as Box<dyn FnMut(_)>);
        self.inner
            .borrow()
            .canvas
            .add_event_listener_with_callback("mousemove", handler.as_ref().unchecked_ref())?;
        handler.forget();

        Ok(())
    }*/
}

#[wasm_bindgen]
impl WebGameRender {
    pub fn render(&self) {
        //log::info!("render game");
    }

    pub fn update(&self) {}
}
