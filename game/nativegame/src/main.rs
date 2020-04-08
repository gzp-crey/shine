use shine_game::{wgpu, GameRender};
use tokio::runtime::Runtime;
use winit::{
    event,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

async fn run(event_loop: EventLoop<()>, window: Window) {
    let surface = wgpu::Surface::create(&window);
    let mut game_view = GameRender::new(surface).await.unwrap();

    let mut size: (u32, u32) = window.inner_size().into();

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
                            virtual_keycode: Some(event::VirtualKeyCode::Escape),
                            state: event::ElementState::Pressed,
                            ..
                        },
                    ..
                }
                | event::WindowEvent::CloseRequested => {
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
        .filter_module("shine-ecs", log::LevelFilter::Debug)
        .filter_module("shine-game", log::LevelFilter::Trace)
        .try_init();
    let mut rt = Runtime::new().unwrap();

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
    rt.block_on(run(event_loop, window));
}
