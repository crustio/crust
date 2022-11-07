// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: GPL-3.0-only

#![deny(warnings)]
use crate as pallet_chainbridge;
use frame_support::{
    assert_noop,
    assert_ok,
};
use pallet_chainbridge::{
    derive_resource_id,
    mock,
    mock::{
        assert_events,
        new_test_ext,
        new_test_ext_initialized,
        Bridge,
        MockRuntime,
        Origin,
        ProposalLifetime,
        TestChainId,
        ENDOWED_BALANCE,
        RELAYER_A,
        RELAYER_B,
        RELAYER_C,
        TEST_THRESHOLD,
    },
    types::{
        ProposalStatus,
        ProposalVotes,
    },
    Error,
    RelayerThreshold,
    ResourceId,
};
use sp_core::U256;

#[test]
fn derive_ids() {
    let chain = 1;
    let id = [
        0x21, 0x60, 0x5f, 0x71, 0x84, 0x5f, 0x37, 0x2a, 0x9e, 0xd8, 0x42, 0x53,
        0xd2, 0xd0, 0x24, 0xb7, 0xb1, 0x09, 0x99, 0xf4,
    ];
    let r_id = derive_resource_id(chain, &id);
    let expected = [
        0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x21, 0x60,
        0x5f, 0x71, 0x84, 0x5f, 0x37, 0x2a, 0x9e, 0xd8, 0x42, 0x53, 0xd2, 0xd0,
        0x24, 0xb7, 0xb1, 0x09, 0x99, 0xf4, chain,
    ];
    assert_eq!(r_id, expected);
}

#[test]
fn complete_proposal_approved() {
    let mut prop = ProposalVotes {
        votes_for: vec![1, 2],
        votes_against: vec![3],
        status: ProposalStatus::Initiated,
        expiry: ProposalLifetime::get(),
    };

    prop.try_to_complete(2, 3);
    assert_eq!(prop.status, ProposalStatus::Approved);
}

#[test]
fn complete_proposal_rejected() {
    let mut prop = ProposalVotes {
        votes_for: vec![1],
        votes_against: vec![2, 3],
        status: ProposalStatus::Initiated,
        expiry: ProposalLifetime::get(),
    };

    prop.try_to_complete(2, 3);
    assert_eq!(prop.status, ProposalStatus::Rejected);
}

#[test]
fn complete_proposal_bad_threshold() {
    let mut prop = ProposalVotes {
        votes_for: vec![1, 2],
        votes_against: vec![],
        status: ProposalStatus::Initiated,
        expiry: ProposalLifetime::get(),
    };

    prop.try_to_complete(3, 2);
    assert_eq!(prop.status, ProposalStatus::Initiated);

    let mut prop = ProposalVotes {
        votes_for: vec![],
        votes_against: vec![1, 2],
        status: ProposalStatus::Initiated,
        expiry: ProposalLifetime::get(),
    };

    prop.try_to_complete(3, 2);
    assert_eq!(prop.status, ProposalStatus::Initiated);
}

#[test]
fn setup_resources() {
    new_test_ext().execute_with(|| {
        let id: ResourceId = [1; 32];
        let method = "Pallet.do_something".as_bytes().to_vec();
        let method2 = "Pallet.do_somethingElse".as_bytes().to_vec();

        assert_ok!(Bridge::set_resource(Origin::root(), id, method.clone()));
        assert_eq!(Bridge::resources(id), Some(method));

        assert_ok!(Bridge::set_resource(Origin::root(), id, method2.clone()));
        assert_eq!(Bridge::resources(id), Some(method2));

        assert_ok!(Bridge::remove_resource(Origin::root(), id));
        assert_eq!(Bridge::resources(id), None);
    })
}

#[test]
fn whitelist_chain() {
    new_test_ext().execute_with(|| {
        assert!(!Bridge::chain_whitelisted(0));

        assert_ok!(Bridge::whitelist_chain(Origin::root(), 0));
        assert_noop!(
            Bridge::whitelist_chain(Origin::root(), TestChainId::get()),
            Error::<MockRuntime>::InvalidChainId
        );

        assert_events(vec![mock::Event::Bridge(pallet_chainbridge::Event::<
            MockRuntime,
        >::ChainWhitelisted(0))]);
    })
}

#[test]
fn set_get_threshold() {
    new_test_ext().execute_with(|| {
        assert_eq!(<RelayerThreshold::<MockRuntime>>::get(), 1);

        assert_ok!(Bridge::set_threshold(Origin::root(), TEST_THRESHOLD));
        assert_eq!(<RelayerThreshold::<MockRuntime>>::get(), TEST_THRESHOLD);

        assert_ok!(Bridge::set_threshold(Origin::root(), 5));
        assert_eq!(<RelayerThreshold::<MockRuntime>>::get(), 5);

        assert_events(vec![
            mock::Event::Bridge(
                pallet_chainbridge::Event::<MockRuntime>::RelayerThresholdChanged(
                    TEST_THRESHOLD,
                ),
            ),
            mock::Event::Bridge(
                pallet_chainbridge::Event::<MockRuntime>::RelayerThresholdChanged(5),
            ),
        ]);
    })
}

#[test]
fn asset_transfer_success() {
    new_test_ext().execute_with(|| {
        let dest_id = 2;
        let to = vec![2];
        let resource_id = [1; 32];
        let metadata = vec![];
        let amount = 100;
        let token_id = vec![1, 2, 3, 4];

        assert_ok!(Bridge::set_threshold(Origin::root(), TEST_THRESHOLD,));

        assert_ok!(Bridge::whitelist_chain(Origin::root(), dest_id.clone()));
        assert_ok!(Bridge::transfer_fungible(
            dest_id.clone(),
            resource_id.clone(),
            to.clone(),
            amount.into()
        ));
        assert_events(vec![
            mock::Event::Bridge(
                pallet_chainbridge::Event::<MockRuntime>::ChainWhitelisted(
                    dest_id.clone(),
                ),
            ),
            mock::Event::Bridge(
                pallet_chainbridge::Event::<MockRuntime>::FungibleTransfer(
                    dest_id.clone(),
                    1,
                    resource_id.clone(),
                    amount.into(),
                    to.clone(),
                ),
            ),
        ]);

        assert_ok!(Bridge::transfer_nonfungible(
            dest_id.clone(),
            resource_id.clone(),
            token_id.clone(),
            to.clone(),
            metadata.clone()
        ));
        assert_events(vec![mock::Event::Bridge(pallet_chainbridge::Event::<
            MockRuntime,
        >::NonFungibleTransfer(
            dest_id.clone(),
            2,
            resource_id.clone(),
            token_id,
            to.clone(),
            metadata.clone(),
        ))]);

        assert_ok!(Bridge::transfer_generic(
            dest_id.clone(),
            resource_id.clone(),
            metadata.clone()
        ));
        assert_events(vec![mock::Event::Bridge(pallet_chainbridge::Event::<
            MockRuntime,
        >::GenericTransfer(
            dest_id.clone(),
            3,
            resource_id,
            metadata,
        ))]);
    })
}

#[test]
fn asset_transfer_invalid_chain() {
    new_test_ext().execute_with(|| {
        let chain_id = 2;
        let bad_dest_id = 3;
        let resource_id = [4; 32];

        assert_ok!(Bridge::whitelist_chain(Origin::root(), chain_id.clone()));
        assert_events(vec![mock::Event::Bridge(pallet_chainbridge::Event::<
            MockRuntime,
        >::ChainWhitelisted(
            chain_id.clone()
        ))]);

        assert_noop!(
            Bridge::transfer_fungible(
                bad_dest_id,
                resource_id.clone(),
                vec![],
                U256::zero()
            ),
            Error::<MockRuntime>::ChainNotWhitelisted
        );

        assert_noop!(
            Bridge::transfer_nonfungible(
                bad_dest_id,
                resource_id.clone(),
                vec![],
                vec![],
                vec![]
            ),
            Error::<MockRuntime>::ChainNotWhitelisted
        );

        assert_noop!(
            Bridge::transfer_generic(bad_dest_id, resource_id.clone(), vec![]),
            Error::<MockRuntime>::ChainNotWhitelisted
        );
    })
}

#[test]
fn add_remove_relayer() {
    new_test_ext().execute_with(|| {
        assert_ok!(Bridge::set_threshold(Origin::root(), TEST_THRESHOLD,));
        assert_eq!(Bridge::relayer_count(), 0);

        assert_ok!(Bridge::add_relayer(Origin::root(), RELAYER_A));
        assert_ok!(Bridge::add_relayer(Origin::root(), RELAYER_B));
        assert_ok!(Bridge::add_relayer(Origin::root(), RELAYER_C));
        assert_eq!(Bridge::relayer_count(), 3);

        // Already exists
        assert_noop!(
            Bridge::add_relayer(Origin::root(), RELAYER_A),
            Error::<MockRuntime>::RelayerAlreadyExists
        );

        // Confirm removal
        assert_ok!(Bridge::remove_relayer(Origin::root(), RELAYER_B));
        assert_eq!(Bridge::relayer_count(), 2);
        assert_noop!(
            Bridge::remove_relayer(Origin::root(), RELAYER_B),
            Error::<MockRuntime>::RelayerInvalid
        );
        assert_eq!(Bridge::relayer_count(), 2);

        assert_events(vec![
            mock::Event::Bridge(
                pallet_chainbridge::Event::<MockRuntime>::RelayerAdded(
                    RELAYER_A,
                ),
            ),
            mock::Event::Bridge(
                pallet_chainbridge::Event::<MockRuntime>::RelayerAdded(
                    RELAYER_B,
                ),
            ),
            mock::Event::Bridge(
                pallet_chainbridge::Event::<MockRuntime>::RelayerAdded(
                    RELAYER_C,
                ),
            ),
            mock::Event::Bridge(
                pallet_chainbridge::Event::<MockRuntime>::RelayerRemoved(
                    RELAYER_B,
                ),
            ),
        ]);
    })
}

fn make_proposal(r: Vec<u8>) -> mock::Call {
    mock::Call::System(frame_system::Call::remark { remark: r })
}

#[test]
fn create_sucessful_proposal() {
    let src_id = 1;
    let r_id = derive_resource_id(src_id, b"remark");

    new_test_ext_initialized(src_id, r_id, b"System.remark".to_vec())
        .execute_with(|| {
            let prop_id = 1;
            let proposal = make_proposal(vec![10]);

            // Create proposal (& vote)
            assert_ok!(Bridge::acknowledge_proposal(
                Origin::signed(RELAYER_A),
                prop_id,
                src_id,
                r_id,
                Box::new(proposal.clone())
            ));
            let prop =
                Bridge::get_votes(src_id, (prop_id.clone(), proposal.clone()))
                    .unwrap();
            let expected = ProposalVotes {
                votes_for: vec![RELAYER_A],
                votes_against: vec![],
                status: ProposalStatus::Initiated,
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
            let prop =
                Bridge::get_votes(src_id, (prop_id.clone(), proposal.clone()))
                    .unwrap();
            let expected = ProposalVotes {
                votes_for: vec![RELAYER_A],
                votes_against: vec![RELAYER_B],
                status: ProposalStatus::Initiated,
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
            let prop =
                Bridge::get_votes(src_id, (prop_id.clone(), proposal.clone()))
                    .unwrap();
            let expected = ProposalVotes {
                votes_for: vec![RELAYER_A, RELAYER_C],
                votes_against: vec![RELAYER_B],
                status: ProposalStatus::Approved,
                expiry: ProposalLifetime::get() + 1,
            };
            assert_eq!(prop, expected);

            assert_events(vec![
                mock::Event::Bridge(
                    pallet_chainbridge::Event::<MockRuntime>::VoteFor(
                        src_id, prop_id, RELAYER_A,
                    ),
                ),
                mock::Event::Bridge(
                    pallet_chainbridge::Event::<MockRuntime>::VoteAgainst(
                        src_id, prop_id, RELAYER_B,
                    ),
                ),
                mock::Event::Bridge(
                    pallet_chainbridge::Event::<MockRuntime>::VoteFor(
                        src_id, prop_id, RELAYER_C,
                    ),
                ),
                mock::Event::Bridge(
                    pallet_chainbridge::Event::<MockRuntime>::ProposalApproved(
                        src_id, prop_id,
                    ),
                ),
                mock::Event::Bridge(
                    pallet_chainbridge::Event::<MockRuntime>::ProposalSucceeded(
                        src_id, prop_id,
                    ),
                ),
            ]);
        })
}

#[test]
fn create_unsucessful_proposal() {
    let src_id = 1;
    let r_id = derive_resource_id(src_id, b"transfer");

    new_test_ext_initialized(src_id, r_id, b"System.remark".to_vec())
        .execute_with(|| {
            let prop_id = 1;
            let proposal = make_proposal(vec![11]);

            // Create proposal (& vote)
            assert_ok!(Bridge::acknowledge_proposal(
                Origin::signed(RELAYER_A),
                prop_id,
                src_id,
                r_id,
                Box::new(proposal.clone())
            ));
            let prop =
                Bridge::get_votes(src_id, (prop_id.clone(), proposal.clone()))
                    .unwrap();
            let expected = ProposalVotes {
                votes_for: vec![RELAYER_A],
                votes_against: vec![],
                status: ProposalStatus::Initiated,
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
            let prop =
                Bridge::get_votes(src_id, (prop_id.clone(), proposal.clone()))
                    .unwrap();
            let expected = ProposalVotes {
                votes_for: vec![RELAYER_A],
                votes_against: vec![RELAYER_B],
                status: ProposalStatus::Initiated,
                expiry: ProposalLifetime::get() + 1,
            };
            assert_eq!(prop, expected);

            // Third relayer votes against
            assert_ok!(Bridge::reject_proposal(
                Origin::signed(RELAYER_C),
                prop_id,
                src_id,
                r_id,
                Box::new(proposal.clone())
            ));
            let prop =
                Bridge::get_votes(src_id, (prop_id.clone(), proposal.clone()))
                    .unwrap();
            let expected = ProposalVotes {
                votes_for: vec![RELAYER_A],
                votes_against: vec![RELAYER_B, RELAYER_C],
                status: ProposalStatus::Rejected,
                expiry: ProposalLifetime::get() + 1,
            };
            assert_eq!(prop, expected);

            assert_eq!(mock::Balances::free_balance(RELAYER_B), 0);
            assert_eq!(
                mock::Balances::free_balance(Bridge::account_id()),
                ENDOWED_BALANCE
            );

            assert_events(vec![
                mock::Event::Bridge(
                    pallet_chainbridge::Event::<MockRuntime>::VoteFor(
                        src_id, prop_id, RELAYER_A,
                    ),
                ),
                mock::Event::Bridge(
                    pallet_chainbridge::Event::<MockRuntime>::VoteAgainst(
                        src_id, prop_id, RELAYER_B,
                    ),
                ),
                mock::Event::Bridge(
                    pallet_chainbridge::Event::<MockRuntime>::VoteAgainst(
                        src_id, prop_id, RELAYER_C,
                    ),
                ),
                mock::Event::Bridge(
                    pallet_chainbridge::Event::<MockRuntime>::ProposalRejected(
                        src_id, prop_id,
                    ),
                ),
            ]);
        })
}

#[test]
fn execute_after_threshold_change() {
    let src_id = 1;
    let r_id = derive_resource_id(src_id, b"transfer");

    new_test_ext_initialized(src_id, r_id, b"System.remark".to_vec())
        .execute_with(|| {
            let prop_id = 1;
            let proposal = make_proposal(vec![11]);

            // Create proposal (& vote)
            assert_ok!(Bridge::acknowledge_proposal(
                Origin::signed(RELAYER_A),
                prop_id,
                src_id,
                r_id,
                Box::new(proposal.clone())
            ));
            let prop =
                Bridge::get_votes(src_id, (prop_id.clone(), proposal.clone()))
                    .unwrap();
            let expected = ProposalVotes {
                votes_for: vec![RELAYER_A],
                votes_against: vec![],
                status: ProposalStatus::Initiated,
                expiry: ProposalLifetime::get() + 1,
            };
            assert_eq!(prop, expected);

            // Change threshold
            assert_ok!(Bridge::set_threshold(Origin::root(), 1));

            // Attempt to execute
            assert_ok!(Bridge::eval_vote_state(
                Origin::signed(RELAYER_A),
                prop_id,
                src_id,
                Box::new(proposal.clone())
            ));

            let prop =
                Bridge::get_votes(src_id, (prop_id.clone(), proposal.clone()))
                    .unwrap();
            let expected = ProposalVotes {
                votes_for: vec![RELAYER_A],
                votes_against: vec![],
                status: ProposalStatus::Approved,
                expiry: ProposalLifetime::get() + 1,
            };
            assert_eq!(prop, expected);

            assert_eq!(mock::Balances::free_balance(RELAYER_B), 0);
            assert_eq!(
                mock::Balances::free_balance(Bridge::account_id()),
                ENDOWED_BALANCE
            );

            assert_events(vec![
                mock::Event::Bridge(pallet_chainbridge::Event::<MockRuntime>::VoteFor(
                    src_id, prop_id, RELAYER_A,
                )),
                mock::Event::Bridge(
                    pallet_chainbridge::Event::<MockRuntime>::RelayerThresholdChanged(1),
                ),
                mock::Event::Bridge(
                    pallet_chainbridge::Event::<MockRuntime>::ProposalApproved(
                        src_id, prop_id,
                    ),
                ),
                mock::Event::Bridge(
                    pallet_chainbridge::Event::<MockRuntime>::ProposalSucceeded(
                        src_id, prop_id,
                    ),
                ),
            ]);
        })
}

#[test]
fn proposal_expires() {
    let src_id = 1;
    let r_id = derive_resource_id(src_id, b"remark");

    new_test_ext_initialized(src_id, r_id, b"System.remark".to_vec())
        .execute_with(|| {
            let prop_id = 1;
            let proposal = make_proposal(vec![10]);

            // Create proposal (& vote)
            assert_ok!(Bridge::acknowledge_proposal(
                Origin::signed(RELAYER_A),
                prop_id,
                src_id,
                r_id,
                Box::new(proposal.clone())
            ));
            let prop =
                Bridge::get_votes(src_id, (prop_id.clone(), proposal.clone()))
                    .unwrap();
            let expected = ProposalVotes {
                votes_for: vec![RELAYER_A],
                votes_against: vec![],
                status: ProposalStatus::Initiated,
                expiry: ProposalLifetime::get() + 1,
            };
            assert_eq!(prop, expected);

            // Increment enough blocks such that now == expiry
            mock::System::set_block_number(ProposalLifetime::get() + 1);

            // Attempt to submit a vote should fail
            assert_noop!(
                Bridge::reject_proposal(
                    Origin::signed(RELAYER_B),
                    prop_id,
                    src_id,
                    r_id,
                    Box::new(proposal.clone())
                ),
                Error::<MockRuntime>::ProposalExpired
            );

            // Proposal state should remain unchanged
            let prop =
                Bridge::get_votes(src_id, (prop_id.clone(), proposal.clone()))
                    .unwrap();
            let expected = ProposalVotes {
                votes_for: vec![RELAYER_A],
                votes_against: vec![],
                status: ProposalStatus::Initiated,
                expiry: ProposalLifetime::get() + 1,
            };
            assert_eq!(prop, expected);

            // eval_vote_state should have no effect
            assert_noop!(
                Bridge::eval_vote_state(
                    Origin::signed(RELAYER_C),
                    prop_id,
                    src_id,
                    Box::new(proposal.clone())
                ),
                Error::<MockRuntime>::ProposalExpired
            );
            let prop =
                Bridge::get_votes(src_id, (prop_id.clone(), proposal.clone()))
                    .unwrap();
            let expected = ProposalVotes {
                votes_for: vec![RELAYER_A],
                votes_against: vec![],
                status: ProposalStatus::Initiated,
                expiry: ProposalLifetime::get() + 1,
            };
            assert_eq!(prop, expected);

            assert_events(vec![mock::Event::Bridge(
                pallet_chainbridge::Event::<MockRuntime>::VoteFor(
                    src_id, prop_id, RELAYER_A,
                ),
            )]);
        })
}
