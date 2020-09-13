use shine_ecs::{
    resources::{MultiRes, MultiResMut, Res, ResMut, ResourceName, Resources},
    scheduler::{IntoSystemConfiguration, Schedule},
};
use std::str::FromStr;

mod utils;

fn sys0() {
    log::info!("sys0");
}

fn sys3(r1: Res<usize>, r2: ResMut<String>, r3: Res<u8>) {
    log::info!("sys3 {:?}", &*r1);
    assert!(*r1 == 1);
    log::info!("sys3 {:?}", &*r2);
    assert!(&*r2 == "string");
    log::info!("sys3 {:?}", &*r3);
    assert!(*r3 == 3);
}

fn sys4(r1: Res<usize>, r2: ResMut<String>, r3: MultiRes<u8>, r4: MultiResMut<u16>) {
    log::info!("sys4 {:?}", &*r1);
    assert!(*r1 == 1);
    log::info!("sys4 {:?}", &*r2);
    assert!(&*r2 == "string");
    assert!(r3.len() == 3);
    log::info!("sys4 {:?}", r3[0]);
    assert!(r3[0] == 5);
    log::info!("sys4 {:?}", r3[1]);
    assert!(r3[1] == 3);
    log::info!("sys4 {:?}", r3[2]);
    assert!(r3[2] == 6);

    assert!(r4.len() == 1);
    log::info!("sys4 {:?}", r4[0]);
    assert!(r4[0] == 4);
}

#[test]
fn resource_access() {
    utils::init_logger();

    let mut resources = Resources::default();
    log::info!("registering resources...");
    resources.insert(None, 1usize);
    resources.insert(None, 2u32);
    resources.insert(None, 3u8);
    resources.insert(Some(ResourceName::from_str("five").unwrap()), 5u8);
    resources.insert(Some(ResourceName::from_str("six").unwrap()), 6u8);
    resources.insert(None, 4u16);
    resources.insert(None, "string".to_string());

    log::info!("registering systems...");
    let mut sh = Schedule::new();

    sh.schedule(sys0.system());
    sh.schedule(sys3.system());
    sh.schedule(
        sys4.system()
            .with_resources::<MultiRes<u8>>(&[
                Some(ResourceName::from_str("five").unwrap()),
                None,
                Some(ResourceName::from_str("six").unwrap()),
            ])
            .with_resources::<MultiResMut<u16>>(&[None]),
    );

    log::info!("runing systems...");
    sh.run(&mut resources);
}
