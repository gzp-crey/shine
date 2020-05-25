#![feature(async_closure)]

use shine_game::{assets::Url, render::Surface, wgpu, world, Config, GameRender};
use tokio::runtime::{Handle as RuntimeHandle, Runtime};
use winit::{
    event,
    event_loop::{ControlFlow, EventLoop},
};

#[cfg(windows)]
use winit::platform::windows::EventLoopExtWindows;

type UserEventType = ();

async fn run() {
    //tokio::task::spawn(async move || logic());

    tokio::task::spawn_blocking(|| {
        let rt = RuntimeHandle::current();

        let event_loop: EventLoop<UserEventType> = EventLoop::new_any_thread();
        let window = {
            let mut builder = winit::window::WindowBuilder::new();
            builder = builder.with_title("Shine");
            builder.build(&event_loop).unwrap()
        };

        let wgpu_instance = wgpu::Instance::new();
        let surface = unsafe { wgpu_instance.create_surface(&window) };
        let mut size: (u32, u32) = window.inner_size().into();

        let surface = Surface::new(surface, size);
        let config = Config::new().unwrap();
        let asset_base = Url::parse(&config.asset_base).unwrap();
        let test1_url = asset_base.join("test_worlds/test1/test.wrld ").unwrap();
        let test2_url = asset_base.join("test_worlds/test2/test.wrld ").unwrap();
        let test3_url = asset_base.join("test_worlds/test3/test.wrld ").unwrap();
        let mut game = rt.block_on(GameRender::new(config, wgpu_instance, surface)).unwrap();

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
                    if let Err(err) = game.render(size) {
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
                        Some(event::VirtualKeyCode::Key0) => rt.block_on(world::unload_world(&mut game)).unwrap(),
                        Some(event::VirtualKeyCode::Key1) => {
                            rt.block_on(world::load_world(&test1_url, &mut game)).unwrap()
                        }
                        Some(event::VirtualKeyCode::Key2) => {
                            rt.block_on(world::load_world(&test2_url, &mut game)).unwrap()
                        }
                        Some(event::VirtualKeyCode::Key3) => {
                            rt.block_on(world::load_world(&test3_url, &mut game)).unwrap()
                        }
                        Some(event::VirtualKeyCode::U) => game.update(),
                        Some(event::VirtualKeyCode::G) => game.gc_all(),
                        _ => {}
                    },
                    event::WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {
                        game.update();
                    }
                },
                _ => {}
            }

            if &ControlFlow::Exit == control_flow {
                rt.block_on(world::unload_world(&mut game)).unwrap();
            }
        })
    })
    .await
    .unwrap();
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
