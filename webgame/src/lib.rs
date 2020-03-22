use console_error_panic_hook;
use shine_game::GameRender;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_logger;
use web_sys::HtmlCanvasElement;

mod inputmapper;

use inputmapper::{WebInputEvent, WebInputMapper};

struct Inner {
    canvas: HtmlCanvasElement,
    render: GameRender,
    input_mapper: WebInputMapper,
}

#[wasm_bindgen]
pub struct WebGame {
    inner: Rc<RefCell<Inner>>,
}

impl WebGame {
    fn attach_mouse_down_handler(&mut self) -> Result<(), JsValue> {
        let inner = self.inner.clone();
        let handler = move |event: web_sys::MouseEvent| {
            let Inner {
                ref mut render,
                ref input_mapper,
                ref canvas,
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
    }
}

#[wasm_bindgen]
impl WebGame {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<WebGame, JsValue> {
        wasm_logger::init(wasm_logger::Config::default());
        console_error_panic_hook::set_once();

        let canvas = {
            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();
            let canvas = document.get_element_by_id("gameCanvas").unwrap();
            canvas.dyn_into::<web_sys::HtmlCanvasElement>()?
        };

        let inner = Rc::new(RefCell::new(Inner {
            canvas,
            input_mapper: WebInputMapper::new(),
            render: GameRender::new(),
        }));

        let mut game = WebGame { inner };

        game.attach_mouse_down_handler()?;
        Ok(game)
    }

    pub fn render(&self) {
        //log::info!("render game");
    }

    pub fn update(&self) {}
}
