#![feature(async_closure)]

use shine_game::{render::Surface, wgpu, world, Config, GameRender};
use tokio::runtime::Runtime;
use winit::{
    event,
    event_loop::{ControlFlow, EventLoop},
};

async fn run() {
    let event_loop = EventLoop::new();
    let window = {
        let mut builder = winit::window::WindowBuilder::new();
        builder = builder.with_title("Shine");
        #[cfg(windows_OFF)] //TODO
        {
            use winit::platform::windows::WindowBuilderExtWindows;
            builder = builder.with_no_redirection_bitmap(true);
        }
        builder.build(&event_loop).unwrap()
    };

    let wgpu_instance = wgpu::Instance::new();
    let surface = unsafe { wgpu_instance.create_surface(&window) };
    let mut size: (u32, u32) = window.inner_size().into();

    let surface = Surface::new(surface, size);
    let config = Config::new().unwrap();
    let mut game_view = GameRender::new(config, wgpu_instance, surface).await.unwrap();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            event::Event::MainEventsCleared => window.request_redraw(),
            event::Event::WindowEvent {
                event: event::WindowEvent::Resized(s),
                ..
            } => {
                size = s.into();
            }
            event::Event::RedrawRequested(_) => {
                if let Err(err) = game_view.render(size) {
                    log::warn!("render failed: {}", err);
                }
            }
            event::Event::WindowEvent { event, .. } => match event {
                event::WindowEvent::KeyboardInput {
                    input:
                        event::KeyboardInput {
                            virtual_keycode,
                            state: event::ElementState::Pressed,
                            ..
                        },
                    ..
                } => match virtual_keycode {
                    Some(event::VirtualKeyCode::Escape) => *control_flow = ControlFlow::Exit,
                    Some(event::VirtualKeyCode::Key0) => world::unregister(&mut game_view).unwrap(),
                    Some(event::VirtualKeyCode::Key1) => world::register_test1(&mut game_view).unwrap(),
                    Some(event::VirtualKeyCode::Key2) => world::register_test2(&mut game_view).unwrap(),
                    Some(event::VirtualKeyCode::Key3) => world::register_test3(&mut game_view).unwrap(),

                    Some(event::VirtualKeyCode::U) => game_view.update(),
                    Some(event::VirtualKeyCode::G) => game_view.gc_all(),
                    _ => {}
                },
                event::WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => {
                    game_view.update();
                }
            },
            _ => {}
        }
    });
}

fn main() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .filter_module("shine_ecs", log::LevelFilter::Trace)
        .filter_module("shine_game", log::LevelFilter::Trace)
        .filter_module("wgpu_core::command::allocator", log::LevelFilter::Warn)
        .try_init();
    let mut rt = Runtime::new().unwrap();

    rt.block_on(run());
}
