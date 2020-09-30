#![feature(async_closure)]

use shine_game::{
    app::{App, AppError, Config},
    assets::{AssetPlugin, Url},
    game::test1::Test1,
    input::InputPlugin,
    render::{RenderPlugin, Surface},
    wgpu, World,
};
use std::time::{Duration, Instant};
use tokio::runtime::{Handle as RuntimeHandle, Runtime};
use winit::{
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
};

#[cfg(windows)]
use winit::platform::windows::EventLoopExtWindows;

const TARGET_FPS: u64 = 30;

#[derive(Debug, Clone)]
pub enum CustomEvent {
    SyncUpdate,
}

async fn logic(event_loop_proxy: EventLoopProxy<CustomEvent>) {
    loop {
        tokio::time::delay_for(Duration::from_millis(100)).await;

        if let Err(_) = event_loop_proxy.send_event(CustomEvent::SyncUpdate) {
            break;
        }
    }
}

async fn run() {
    tokio::task::spawn_blocking(|| {
        let rt = RuntimeHandle::current();

        let event_loop: EventLoop<CustomEvent> = EventLoop::new_any_thread();
        let window = {
            let mut builder = winit::window::WindowBuilder::new();
            builder = builder.with_title("Shine");
            builder.build(&event_loop).unwrap()
        };

        let wgpu_instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { wgpu_instance.create_surface(&window) };
        let mut size: (u32, u32) = window.inner_size().into();

        let surface = Surface::new(surface, size);
        let config = Config::new().unwrap();
        let test1_url = Url::parse("game://games/test1/test.game").unwrap();
        let test2_url = Url::parse("game://games/test2/test.game").unwrap();
        let test3_url = Url::parse("game://games/test3/test.game").unwrap();
        let test4_url = Url::parse("game://games/test4/test.game").unwrap();
        let test5_url = Url::parse("game://games/test5/test.game").unwrap();
        let mut app = rt
            .block_on(async move {
                let mut app = App::default();
                app.world.add_asset_plugin(config.asset.clone()).await?;
                app.world
                    .add_render_plugin(config.render.clone(), wgpu_instance, surface)
                    .await?;
                app.world.add_input_plugin().await?;
                Ok::<_, AppError>(app)
            })
            .unwrap();

        let event_proxy = event_loop.create_proxy();
        tokio::task::spawn(logic(event_proxy));

        rt.block_on(app.load_game_from_url(&test1_url)).unwrap();

        let mut prev_render_time = Instant::now();
        let mut is_closing = false;
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match &event {
                Event::MainEventsCleared => window.request_redraw(),
                Event::WindowEvent {
                    event: WindowEvent::Resized(s),
                    ..
                } => {
                    size = (*s).into();
                }
                Event::UserEvent(_event) => {
                    //log::info!("User event: {:?}", event);
                }
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::KeyboardInput { input, .. } => {
                        let _ = app.world.inject_input(input);
                        if input.state == ElementState::Pressed {
                            let alt = input.modifiers.shift();
                            match input.virtual_keycode {
                                Some(VirtualKeyCode::Escape) => *control_flow = ControlFlow::Exit,
                                Some(VirtualKeyCode::Key0) => rt.block_on(app.unload_game()).unwrap(),
                                Some(VirtualKeyCode::Key1) => rt.block_on(app.load_game_from_url(&test1_url)).unwrap(),
                                Some(VirtualKeyCode::Key2) => rt.block_on(app.load_game_from_url(&test2_url)).unwrap(),
                                Some(VirtualKeyCode::Key3) => rt.block_on(app.load_game_from_url(&test3_url)).unwrap(),
                                Some(VirtualKeyCode::Key4) => rt.block_on(app.load_game_from_url(&test4_url)).unwrap(),
                                Some(VirtualKeyCode::Key5) => rt.block_on(app.load_game_from_url(&test5_url)).unwrap(),
                                _ => {}
                            }
                        }
                    }
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {
                        //world.update();
                    }
                },
                _ => {}
            }

            if is_closing {
                log::trace!("close events: {:?}", event);
                return;
            }

            if control_flow == &ControlFlow::Exit {
                rt.block_on(app.unload_game()).unwrap();
                is_closing = true;
                return;
            }

            let now = Instant::now();
            let elapsed_time = now.duration_since(prev_render_time).as_millis() as i64;
            let wait_millis = ((1000 / TARGET_FPS) as i64) - elapsed_time;
            if wait_millis > 0 {
                // we have some time left from rendering
                let new_inst = prev_render_time + std::time::Duration::from_millis(wait_millis as u64);
                *control_flow = ControlFlow::WaitUntil(new_inst);
            } else {
                // no time left
                if let Err(err) = app.world.render(size) {
                    log::warn!("Failed to render: {:?}", err);
                }
                *control_flow = ControlFlow::Poll;
                prev_render_time = now;
            }
        })
    })
    .await
    .unwrap()
}

fn main() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .filter_module("shine_native", log::LevelFilter::Trace)
        .filter_module("shine_ecs", log::LevelFilter::Trace)
        .filter_module("shine_game", log::LevelFilter::Trace)
        .filter_module("shine_input", log::LevelFilter::Info)
        .filter_module("gfx_backend_vulkan", log::LevelFilter::Trace)
        .try_init();
    let mut rt = Runtime::new().unwrap();

    rt.block_on(run());
}
