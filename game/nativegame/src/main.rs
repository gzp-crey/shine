use winit::{
    event,
    event_loop::{ControlFlow, EventLoop},
    window::Window
};
use shine_game::{GameRender, wgpu};
use tokio::runtime::Runtime;

async fn run(event_loop: EventLoop<()>, window: Window) {
    let surface = wgpu::Surface::create(&window);
    let mut gameView = GameRender::new(surface).await.unwrap();

    let mut size : (u32,u32) = window.inner_size().into();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            event::Event::MainEventsCleared => window.request_redraw(),
            event::Event::WindowEvent { event: event::WindowEvent::Resized(s), .. } => {
                size = s.into();
            }
            event::Event::RedrawRequested(_) => {
                gameView.render(size);
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
                    gameView.update();
                }
            }
            _ => {}
        }
    });
}

fn main() {
    env_logger::init();
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
