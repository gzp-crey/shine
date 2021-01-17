use shine_ecs::{
    resources::{MultiRes, MultiResMut, Res, ResMut, Resources},
    scheduler::{IntoSystem, Scheduler, TaskGroup, WithMultiRes, WithMultiResMut},
    ECSError,
};

mod utils;

fn sys0() -> Result<TaskGroup, ECSError> {
    log::info!("sys0");
    Ok(TaskGroup::default())
}

fn sys3(r1: Res<usize>, r2: ResMut<String>, r3: Res<u8>) -> Result<TaskGroup, ECSError> {
    log::info!("r1={:?}", &*r1);
    assert!(*r1 == 1);
    log::info!("r2={:?}", &*r2);
    assert!(&*r2 == "string");
    log::info!("r3={:?}", &*r3);
    assert!(*r3 == 3);
    Ok(TaskGroup::default())
}

fn sys4(r1: Res<usize>, r2: ResMut<String>, r3: MultiRes<u8>, r4: MultiResMut<u16>) -> Result<TaskGroup, ECSError> {
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
    Ok(TaskGroup::default())
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

    resources.insert(1usize).unwrap();
    resources.insert(2u32).unwrap();
    resources.insert("string".to_owned()).unwrap();
    resources.insert(3u8).unwrap();
    resources.insert_tagged("five", 5u8).unwrap();
    resources.insert_tagged("six", 6u8).unwrap();
    resources.insert(4u16).unwrap();
    resources.insert_tagged("16", 16u16).unwrap();

    log::info!("registering systems...");
    let mut tasks = TaskGroup::default();
    let mut scheduler = Scheduler::default();

    tasks.add_task(sys0.into_system());
    tasks.add_task(sys3.into_system().with_name(None));
    tasks.add_tasks(Some(
        sys4.into_system()
            .claim_res::<u8, _>(|claim| claim.try_add_tags(&["five", "six"]).unwrap())
            .claim_res_mut::<u16, _>(|claim| claim.try_add_tags(&["16"]).unwrap()),
    ));

    log::info!("runing systems...");
    scheduler.run(&mut resources, &tasks).unwrap();
}
