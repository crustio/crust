use crate::mock::{new_test_ext, run_to_block, Origin, Market};
use frame_support::{
    assert_noop, assert_ok,
    dispatch::DispatchError,
};
use hex;
use market::{StorageOrder, Provision};
use sp_core::H256;

#[test]
fn test_for_storage_order_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_identifier =
        hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let provider = 100;
        let file_size = 16; // should less than provider
        let duration = 360; // file should store at least 30 minutes
        let fee = 10;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();

        assert_ok!(Market::register(Origin::signed(provider), address_info.clone()));
        assert_ok!(Market::place_storage_order(
            Origin::signed(source), provider, fee,
            file_identifier.clone(), file_size, duration
        ));

        let order_id = H256::default();
        assert_eq!(Market::providers(100).unwrap(), Provision {
            address_info,
            file_map: vec![(file_identifier.clone(), order_id.clone())].into_iter().collect()
        });
        assert_eq!(Market::clients(0).unwrap(), vec![order_id.clone()]);
        assert_eq!(Market::storage_orders(order_id).unwrap(), StorageOrder {
            file_identifier,
            file_size: 16,
            created_on: 50,
            expired_on: 410,
            provider: 100,
            client: 0,
            order_status: Default::default()
        });
    });
}