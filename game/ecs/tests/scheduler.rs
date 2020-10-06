use shine_ecs::{
    resources::{NamedRes, NamedResClaim, NamedResMut, NamedResMutClaim, Res, ResMut, ResourceName, Resources},
    scheduler::{IntoSystemBuilder, Schedule},
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

fn sys4(r1: Res<usize>, r2: ResMut<String>, r3: NamedRes<u8>, r4: NamedResMut<u16>) {
    log::info!("sys4 {:?}", &*r1);
    assert!(*r1 == 1);
    log::info!("sys4 {:?}", &*r2);
    assert!(&*r2 == "string");
    assert!(r3.len() == 2);
    log::info!("sys4 {:?}", r3[0]);
    assert!(r3[0] == 5);
    log::info!("sys4 {:?}", r3[1]);
    assert!(r3[1] == 6);

    assert!(r4.len() == 1);
    log::info!("sys4 {:?}", r4[0]);
    assert!(r4[0] == 16);
}

fn dumpt<C>(a: &C) {
    log::error!("{:#?}", std::any::type_name::<C>());
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
    sh.schedule(sys4.system().claim(|claims| {
        {
            let x = claims.get_mut::<NamedResClaim<u8>, _>();
            x.push(ResourceName::from_str("five").unwrap());
            x.push(ResourceName::from_str("six").unwrap());
        }

        {
            let x = claims.get_mut::<NamedResMutClaim<u16>, _>();
            x.push(ResourceName::from_str("16").unwrap());
        }
    }));

    log::info!("runing systems...");
    sh.run(&mut resources);
}
