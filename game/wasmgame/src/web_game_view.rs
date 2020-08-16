use crate::web_window::WebWindow;
use js_sys;
use js_sys::Promise;
use shine_game::{
    assets::Url,
    render::{RenderPlugin, Surface},
    wgpu,
    world::WorldSystem,
    Config, GameError, GameView,
};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::future_to_promise;
use wasm_bindgen_macro::wasm_bindgen;

struct Inner {
    window: WebWindow,
    game_view: GameView,
}

impl Inner {
    async fn load_world(&mut self, url: String) -> Result<JsValue, JsValue> {
        use shine_game::world::WorldData;
        let url =
            Url::parse(&url).map_err(|err| js_sys::Error::new(&format!("Failed to parse world url: {:?}", err)))?;
        let world_data = WorldData::from_url(&self.game_view.assetio, &url)
            .await
            .map_err(|err| js_sys::Error::new(&format!("Failed to download world: {:?}", err)))?;
        match world_data {
            WorldData::Test1(test) => self.game_view.load_world(test),
            WorldData::Test2(test) => self.game_view.load_world(test),
            WorldData::Test3(test) => self.game_view.load_world(test),
            WorldData::Test4(test) => self.game_view.load_world(test),
        }
        .map_err(|err| js_sys::Error::new(&format!("Failed to parse world: {:?}", err)))?;
        Ok(JsValue::UNDEFINED)
    }
}

#[wasm_bindgen]
pub struct WebGameView {
    inner: Rc<RefCell<Inner>>,
}

impl WebGameView {
    pub async fn new(element: &str, id: u32, cfg: &str) -> Result<WebGameView, JsValue> {
        let config = Config::from_str(cfg).map_err(|err| js_sys::Error::new(&format!("{:?}", err)))?;
        let window = WebWindow::from_element_by_id(element, id)?;
        //window.attach_mouse_down_handler()?;

        let wgpu_instance = wgpu::Instance::new();
        let surface = unsafe { wgpu_instance.create_surface(&window) };
        let size: (u32, u32) = window.inner_size().into();
        let game_view = GameView::new(config, wgpu_instance, Surface::new(surface, size))
            .await
            .map_err(|err| js_sys::Error::new(&format!("{:?}", err)))?;

        let inner = Rc::new(RefCell::new(Inner { window, game_view }));

        Ok(WebGameView { inner })
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
impl WebGameView {
    pub fn render(&self) {
        let inner = &mut *self.inner.borrow_mut();
        let size = inner.window.inner_size();
        if let Err(err) = inner.game_view.refresh(size) {
            log::warn!("Failed to render: {:?}", err);
        }
    }

    pub fn load_world(&self, url: String) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move {
            let inner = &mut *inner.borrow_mut();
            inner.load_world(url).await
        })
    }
}
