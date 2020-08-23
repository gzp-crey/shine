#![feature(async_closure)]

use shine_game::{
    assets::Url,
    input::InputPlugin,
    render::Surface,
    wgpu,
    world::{
        test1::Test1World, test2::Test2World, test3::Test3World, test4::Test4World, test5::Test5World, WorldSystem,
    },
    Config, GameError, GameView,
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

fn load_world(rt: &RuntimeHandle, game: &mut GameView, url: &Url, gc: bool) -> Result<(), GameError> {
    use shine_game::world::WorldData;
    let world_data = rt
        .block_on(WorldData::from_url(&game.assetio, url))
        .map_err(|err| GameError::Config(format!("Failed to load world: {}", err)))?;
    match world_data {
        WorldData::Test1(test) => game.load_world::<Test1World>(test)?,
        WorldData::Test2(test) => game.load_world::<Test2World>(test)?,
        WorldData::Test3(test) => game.load_world::<Test3World>(test)?,
        WorldData::Test4(test) => game.load_world::<Test4World>(test)?,
        WorldData::Test5(test) => game.load_world::<Test5World>(test)?,
    }
    if gc {
        game.gc();
    }
    Ok(())
}

fn unload_world(_rt: &RuntimeHandle, game: &mut GameView, gc: bool) -> Result<(), GameError> {
    game.unload_world()?;
    if gc {
        game.gc();
    }
    Ok(())
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
        let test1_url = Url::parse("world://test_worlds/test1/test.wrld").unwrap();
        let test2_url = Url::parse("world://test_worlds/test2/test.wrld").unwrap();
        let test3_url = Url::parse("world://test_worlds/test3/test.wrld").unwrap();
        let test4_url = Url::parse("world://test_worlds/test4/test.wrld").unwrap();
        let test5_url = Url::parse("world://test_worlds/test5/test.wrld").unwrap();
        let mut game = rt.block_on(GameView::new(config, wgpu_instance, surface)).unwrap();

        let event_proxy = event_loop.create_proxy();
        tokio::task::spawn(logic(event_proxy));

        let mut prev_render_time = Instant::now();
        let mut is_closing = false;
        //flame::start("frame");
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
                        let _ = game.inject_input(input);
                        if input.state == ElementState::Pressed {
                            let alt = input.modifiers.shift();
                            match input.virtual_keycode {
                                Some(VirtualKeyCode::Escape) => *control_flow = ControlFlow::Exit,
                                /*Some(VirtualKeyCode::F11) => {
                                    flame::end("frame");
                                    flame::clear();
                                    flame::start("frame");
                                },*/
                                Some(VirtualKeyCode::Key0) => unload_world(&rt, &mut game, alt).unwrap(),
                                Some(VirtualKeyCode::Key1) => load_world(&rt, &mut game, &test1_url, alt).unwrap(),
                                Some(VirtualKeyCode::Key2) => load_world(&rt, &mut game, &test2_url, alt).unwrap(),
                                Some(VirtualKeyCode::Key3) => load_world(&rt, &mut game, &test3_url, alt).unwrap(),
                                Some(VirtualKeyCode::Key4) => load_world(&rt, &mut game, &test4_url, alt).unwrap(),
                                Some(VirtualKeyCode::Key5) => load_world(&rt, &mut game, &test5_url, alt).unwrap(),
                                Some(VirtualKeyCode::F9) => game.gc(),
                                _ => {}
                            }
                        }
                    }
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {
                        //game.update();
                    }
                },
                _ => {}
            }

            if is_closing {
                log::trace!("close events: {:?}", event);
                return;
            }

            if control_flow == &ControlFlow::Exit {
                //flame::end("frame");
                //use std::fs::File;
                //flame::dump_html(&mut File::create("flame-graph.html").unwrap()).unwrap();
                game.unload_world().unwrap();
                game.gc();
                is_closing = true;
                return;
            }

            let now = Instant::now();
            let elapsed_time = now.duration_since(prev_render_time).as_millis() as i64;
            let wait_millis = ((1000 / TARGET_FPS) as i64) - elapsed_time;
            if wait_millis > 0 {
                // we have some time left from rendering
                //log::trace!("wait untils next render: {}", wait_millis);
                let new_inst = prev_render_time + std::time::Duration::from_millis(wait_millis as u64);
                *control_flow = ControlFlow::WaitUntil(new_inst);
            } else {
                // no time left
                //log::trace!("elapsed time since last render: {} ({})", elapsed_time, 1000./(elapsed_time as f64));
                if let Err(err) = game.refresh(size) {
                    log::warn!("Failed to render: {:?}", err);
                }
                //flame::end("frame");
                //flame::start("frame");
                *control_flow = ControlFlow::Poll;
                prev_render_time = now;
            }
        })
    })
    .await
    .unwrap();
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
