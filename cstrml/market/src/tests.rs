use crate::mock::{new_test_ext, run_to_block, Origin, Market};
use frame_support::assert_ok;
use hex;

#[test]
fn test_for_storage_order_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_indetifier = 
        hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let destination = 100;
        let file_size = 16; // should less than destination
        let expired_duration = 16;
        let expired_on = 20;
        let fee = 10;
        assert_ok!(Market::store_storage_order(
            Origin::signed(source.clone()), destination, fee,
            file_indetifier, file_size, expired_duration, expired_on
        ));
    });
}


#[test]
fn test_for_storage_order_should_fail_due_to_file_size() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_indetifier = 
        hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let destination = 100;
        let file_size = 200; // should less than destination
        let expired_duration = 16;
        let expired_on = 20;
        let fee = 10;
        assert!(Market::store_storage_order(
            Origin::signed(source.clone()), destination, fee,
            file_indetifier, file_size, expired_duration, expired_on
        ).is_err());
    });
}

#[test]
fn test_for_storage_order_should_fail_due_to_exist_of_wr() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_indetifier = 
        hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let destination = 400; // Invalid destination. No work report
        let file_size = 200;
        let expired_duration = 16;
        let expired_on = 20;
        let fee = 10;
        assert!(Market::store_storage_order(
            Origin::signed(source.clone()), destination, fee,
            file_indetifier, file_size, expired_duration, expired_on
        ).is_err());
    });
}
