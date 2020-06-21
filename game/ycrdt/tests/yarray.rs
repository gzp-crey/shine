use shine_ycrdt::{YContext, ClientId, YArrayDoc};

mod utils;

#[test]
fn test_delete_insert() {
    utils::init_logger();

    let user0 = YContext::new(ClientId::from(0));
    let user1 = YContext::new(ClientId::from(1));
    let mut array0 = YArrayDoc::new(user0);
    //let mut array1 = YArrayDoc::new(user1);

    log::info!("Does not throw when deleting zero elements with position 0");
    //let _ = array0.remove(0, 0);
    //assert!(array0.delete(1, 1).is_err());

    array0.insert(0, "A");    
    //log::info!("Does not throw when deleting zero elements with valid position 1");
    //array0.delete(1, 0);
    //compare(users)
}
