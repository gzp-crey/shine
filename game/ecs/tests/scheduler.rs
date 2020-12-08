use shine_ecs::{
    resources::{ResourceId, Resources},
    scheduler::{prelude::*, Res, ResMut, Schedule, TagRes, TagResMut},
};

mod utils;

fn sys0() {
    log::info!("sys0");
}

fn sys3(r1: Res<usize>, r2: ResMut<String>, r3: Res<u8>) {
    log::info!("r1={:?}", &*r1);
    assert!(*r1 == 1);
    log::info!("r2={:?}", &*r2);
    assert!(&*r2 == "string");
    log::info!("r3={:?}", &*r3);
    assert!(*r3 == 3);
}

fn sys4(r1: Res<usize>, r2: ResMut<String>, r3: TagRes<u8>, r4: TagResMut<u16>) {
    log::info!("claims: u8: {:?}", r3.claim());
    log::info!("claims: u16: {:?}", r4.claim());
    log::info!("r1={:?}", &*r1);
    assert!(*r1 == 1);
    log::info!("r2={:?}", &*r2);
    assert!(&*r2 == "string");
    assert!(r3.len() == 2);
    log::info!("r3[0]={:?}", r3[0]);
    assert!(r3[0] == 5);
    log::info!("r3[1]={:?}", r3[1]);
    assert!(r3[1] == 6);
    assert!(r4.len() == 1);
    log::info!("r4[0]={:?}", r4[0]);
    assert!(r4[0] == 16);
}

#[test]
fn resource_access() {
    utils::init_logger();

    let mut resources = Resources::default();
    log::info!("registering resources...");
    resources.register_unmanaged::<usize>().unwrap();
    resources.register_unmanaged::<u32>().unwrap();
    resources.register_unmanaged::<u16>().unwrap();
    resources.register_unmanaged::<u8>().unwrap();
    resources.register_unmanaged::<String>().unwrap();

    resources.insert(1usize);
    resources.insert(2u32);
    resources.insert("string".to_string());
    resources.insert(3u8);
    resources.insert_tagged("five", 5u8);
    resources.insert_tagged("six", 6u8);
    resources.insert(4u16);
    resources.insert_tagged("16", 16u16);

    log::info!("registering systems...");
    let mut sh = Schedule::default();

    sh.schedule(sys0.system()).unwrap();
    sh.schedule(sys3.system()).unwrap();
    sh.schedule(
        sys4.system()
            .with_tag::<u8>(&["five", "six"])
            .with_tag_mut::<u16>(&["16"]),
    )
    .unwrap();

    log::info!("runing systems...");
    sh.run(&mut resources).unwrap();
}
