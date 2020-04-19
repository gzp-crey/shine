#![feature(async_closure)]

use shine_game::{render::Surface, wgpu, Config, GameRender};
use tokio;
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

    let surface = wgpu::Surface::create(&window);
    let mut size: (u32, u32) = window.inner_size().into();

    let surface = Surface::new(surface, size);
    let config = Config::new().unwrap();
    let mut game_view = GameRender::new(config, surface).await.unwrap();

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
                game_view.render(size);
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
                    Some(event::VirtualKeyCode::A) => game_view.test(),
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
    })
}

fn main() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .filter_module("shine-ecs", log::LevelFilter::Debug)
        .filter_module("shine-game", log::LevelFilter::Trace)
        .try_init();
    let mut rt = Runtime::new().unwrap();

    rt.block_on(run());
}
