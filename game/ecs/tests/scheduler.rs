use shine_ecs::{
    resources::{NamedRes, NamedResClaim, NamedResMut, NamedResMutClaim, Res, ResMut, ResourceName, Resources},
    scheduler::{IntoSystemBuilder, Schedule},
};
use std::{convert::TryInto, str::FromStr};

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

fn sys4(r1: Res<usize>, r2: ResMut<String>, r3: NamedRes<u8>, r4: NamedResMut<u16>) {
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
    resources.insert(1usize);
    resources.insert(2u32);
    resources.insert(3u8);
    resources.insert_with_name(ResourceName::from_str("five").unwrap(), 5u8);
    resources.insert_with_name(ResourceName::from_str("six").unwrap(), 6u8);
    resources.insert_with_name(ResourceName::from_str("16").unwrap(), 16u16);
    resources.insert(4u16);
    resources.insert("string".to_string());

    log::info!("registering systems...");
    let mut sh = Schedule::default();

    sh.schedule(sys0.system());
    sh.schedule(sys3.system());
    sh.schedule(
        sys4.system()
            .with_claim::<NamedResClaim<u8>, _>(["five", "six"][..].try_into().unwrap())
            .with_claim::<NamedResMutClaim<u16>, _>(["16"][..].try_into().unwrap()),
    );

    log::info!("runing systems...");
    sh.run(&mut resources);
}
