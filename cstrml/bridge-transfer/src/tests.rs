#![cfg(test)]

use super::mock::{
	assert_events, balances, expect_event, new_test_ext, Balances, Bridge,
	BridgeTransfer, Call, Event, Origin, ProposalLifetime, ENDOWED_BALANCE, RELAYER_A,
	RELAYER_B, RELAYER_C, BridgeTokenId
};

use super::*;
use frame_support::assert_ok;

use blake2_rfc;

/// Do a Blake2 128-bit hash and place result in `dest`.
fn blake2_128_into(data: &[u8], dest: &mut [u8; 16]) {
	dest.copy_from_slice(blake2_rfc::blake2b::blake2b(16, &[], data).as_bytes());
}

/// Do a Blake2 128-bit hash and return result.
fn blake2_128(data: &[u8]) -> [u8; 16] {
	let mut r = [0; 16];
	blake2_128_into(data, &mut r);
	r
}

const TEST_THRESHOLD: u32 = 2;

fn make_transfer_proposal(to: u64, amount: u64) -> Call {
	let resource_id = BridgeTokenId::get();
	Call::BridgeTransfer(crate::Call::transfer(to, amount.into(), resource_id))
}

#[test]
fn constant_equality() {
	let r_id = bridge::derive_resource_id(1, &blake2_128(b"CRU"));
	let encoded: [u8; 32] = hex_literal::hex!("000000000000000000000000000000608d1bc9a2d146ebc94667c336721b2801");
	assert_eq!(r_id, encoded);
}

#[test]
fn transfer_native() {
	new_test_ext().execute_with(|| {
		let dest_chain = 0;
		let resource_id = BridgeTokenId::get();
		let amount: u64 = 100;
		let recipient = vec![99];

		assert_ok!(Bridge::whitelist_chain(Origin::root(), dest_chain.clone()));
		assert_ok!(BridgeTransfer::sudo_change_fee(
			Origin::root(),
			2,
			2,
			dest_chain.clone()
		));
		assert_ok!(BridgeTransfer::transfer_native(
			Origin::signed(RELAYER_A),
			amount.clone(),
			recipient.clone(),
			dest_chain,
		));

		expect_event(bridge::RawEvent::FungibleTransfer(
			dest_chain,
			1,
			resource_id,
			(amount - 2).into(),
			recipient,
		));
	})
}

#[test]
fn transfer() {
	new_test_ext().execute_with(|| {
		// Check inital state
		let bridge_id: u64 = Bridge::account_id();
		let resource_id = BridgeTokenId::get();
		assert_eq!(Balances::free_balance(&bridge_id), ENDOWED_BALANCE);
		// Transfer and check result
		assert_ok!(BridgeTransfer::transfer(
			Origin::signed(Bridge::account_id()),
			RELAYER_A,
			10,
			resource_id,
		));
		assert_eq!(Balances::free_balance(&bridge_id), ENDOWED_BALANCE - 10);
		assert_eq!(Balances::free_balance(RELAYER_A), ENDOWED_BALANCE + 10);

		assert_events(vec![Event::balances(balances::Event::Transfer(
			Bridge::account_id(),
			RELAYER_A,
			10,
		))]);
	})
}

#[test]
fn create_sucessful_transfer_proposal() {
	new_test_ext().execute_with(|| {
		let prop_id = 1;
		let src_id = 1;
		let r_id = bridge::derive_resource_id(src_id, b"transfer");
		let resource = b"BridgeTransfer.transfer".to_vec();
		let proposal = make_transfer_proposal(RELAYER_A, 10);

		assert_ok!(Bridge::set_threshold(Origin::root(), TEST_THRESHOLD,));
		assert_ok!(Bridge::add_relayer(Origin::root(), RELAYER_A));
		assert_ok!(Bridge::add_relayer(Origin::root(), RELAYER_B));
		assert_ok!(Bridge::add_relayer(Origin::root(), RELAYER_C));
		assert_ok!(Bridge::whitelist_chain(Origin::root(), src_id));
		assert_ok!(Bridge::set_resource(Origin::root(), r_id, resource));

		// Create proposal (& vote)
		assert_ok!(Bridge::acknowledge_proposal(
			Origin::signed(RELAYER_A),
			prop_id,
			src_id,
			r_id,
			Box::new(proposal.clone())
		));
		let prop = Bridge::votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
		let expected = bridge::ProposalVotes {
			votes_for: vec![RELAYER_A],
			votes_against: vec![],
			status: bridge::ProposalStatus::Initiated,
			expiry: ProposalLifetime::get() + 1,
		};
		assert_eq!(prop, expected);

		// Second relayer votes against
		assert_ok!(Bridge::reject_proposal(
			Origin::signed(RELAYER_B),
			prop_id,
			src_id,
			r_id,
			Box::new(proposal.clone())
		));
		let prop = Bridge::votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
		let expected = bridge::ProposalVotes {
			votes_for: vec![RELAYER_A],
			votes_against: vec![RELAYER_B],
			status: bridge::ProposalStatus::Initiated,
			expiry: ProposalLifetime::get() + 1,
		};
		assert_eq!(prop, expected);

		// Third relayer votes in favour
		assert_ok!(Bridge::acknowledge_proposal(
			Origin::signed(RELAYER_C),
			prop_id,
			src_id,
			r_id,
			Box::new(proposal.clone())
		));
		let prop = Bridge::votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
		let expected = bridge::ProposalVotes {
			votes_for: vec![RELAYER_A, RELAYER_C],
			votes_against: vec![RELAYER_B],
			status: bridge::ProposalStatus::Approved,
			expiry: ProposalLifetime::get() + 1,
		};
		assert_eq!(prop, expected);

		assert_eq!(Balances::free_balance(RELAYER_A), ENDOWED_BALANCE + 10);
		assert_eq!(
			Balances::free_balance(Bridge::account_id()),
			ENDOWED_BALANCE - 10
		);

		assert_events(vec![
			Event::bridge(bridge::RawEvent::VoteFor(src_id, prop_id, RELAYER_A)),
			Event::bridge(bridge::RawEvent::VoteAgainst(src_id, prop_id, RELAYER_B)),
			Event::bridge(bridge::RawEvent::VoteFor(src_id, prop_id, RELAYER_C)),
			Event::bridge(bridge::RawEvent::ProposalApproved(src_id, prop_id)),
			Event::balances(balances::Event::Transfer(
				Bridge::account_id(),
				RELAYER_A,
				10,
			)),
			Event::bridge(bridge::RawEvent::ProposalSucceeded(src_id, prop_id)),
		]);
	})
}
