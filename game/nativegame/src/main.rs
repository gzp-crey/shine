#![feature(async_closure)]

use shine_game::{
    assets::Url,
    render::{GameRender, Surface},
    wgpu, world, Config,
};
use std::time::{Duration, Instant};
use tokio::runtime::{Handle as RuntimeHandle, Runtime};
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
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

        let wgpu_instance = wgpu::Instance::new();
        let surface = unsafe { wgpu_instance.create_surface(&window) };
        let mut size: (u32, u32) = window.inner_size().into();

        let surface = Surface::new(surface, size);
        let config = Config::new().unwrap();
        let test1_url = Url::parse("world://test_worlds/test1/test.wrld").unwrap();
        let test2_url = Url::parse("world://test_worlds/test2/test.wrld").unwrap();
        let test3_url = Url::parse("world://test_worlds/test3/test.wrld").unwrap();
        let test4_url = Url::parse("world://test_worlds/test4/test.wrld").unwrap();
        let mut game = rt.block_on(GameRender::new(config, wgpu_instance, surface)).unwrap();

        let event_proxy = event_loop.create_proxy();
        tokio::task::spawn(logic(event_proxy));

        event_loop.run(move |event, _, control_flow| {
            let start_time = Instant::now();
            *control_flow = ControlFlow::Poll;

            match event {
                Event::MainEventsCleared => window.request_redraw(),
                Event::WindowEvent {
                    event: WindowEvent::Resized(s),
                    ..
                } => {
                    size = s.into();
                }
                Event::RedrawRequested(_) => {
                    if let Err(err) = game.render(size) {
                        log::warn!("render failed: {}", err);
                    }
                }
                Event::UserEvent(ref _event) => {
                    //log::info!("User event: {:?}", event);
                }
                Event::WindowEvent { ref event, .. } => match event {
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode,
                                state: ElementState::Pressed,
                                ..
                            },
                        ..
                    } => match virtual_keycode {
                        Some(VirtualKeyCode::Escape) => *control_flow = ControlFlow::Exit,
                        Some(VirtualKeyCode::Key0) => rt.block_on(world::unload_world(&mut game)).unwrap(),
                        Some(VirtualKeyCode::Key1) => rt.block_on(world::load_world(&test1_url, &mut game)).unwrap(),
                        Some(VirtualKeyCode::Key2) => rt.block_on(world::load_world(&test2_url, &mut game)).unwrap(),
                        Some(VirtualKeyCode::Key3) => rt.block_on(world::load_world(&test3_url, &mut game)).unwrap(),
                        Some(VirtualKeyCode::Key4) => rt.block_on(world::load_world(&test4_url, &mut game)).unwrap(),
                        Some(VirtualKeyCode::G) => game.gc_all(),
                        _ => {}
                    },
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {
                        //game.render();
                    }
                },
                _ => {}
            }

            match control_flow {
                ControlFlow::Exit => rt.block_on(world::unload_world(&mut game)).unwrap(),
                _ => {
                    if let Ok(event) = event.map_nonuser_event::<()>() {
                        let _ = game.inject_input(&event);
                    }

                    let elapsed_time = Instant::now().duration_since(start_time).as_millis() as i64;
                    let wait_millis = ((1000 / TARGET_FPS) as i64) - elapsed_time;
                    if wait_millis > 0 {
                        let new_inst = start_time + std::time::Duration::from_millis(wait_millis as u64);
                        *control_flow = ControlFlow::WaitUntil(new_inst);
                    } else {
                        *control_flow = ControlFlow::Poll;
                    }
                }
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
        .filter_module("wgpu_core", log::LevelFilter::Warn)
        .filter_module("mio", log::LevelFilter::Warn)
        .try_init();
    let mut rt = Runtime::new().unwrap();

    rt.block_on(run());
}
