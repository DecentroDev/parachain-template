use crate::{mock::*, Error::NotDaoMember, NextProposalIndex};
use frame_support::{
	assert_noop, assert_ok,
	traits::{fungibles::Inspect, Currency, Hooks},
};
use pallet_collective::Members;
use pallet_insurances::types::{InsuranceMetadata, InsuranceStatus, InsuranceType};
use sp_runtime::{traits::Hash, BoundedVec};

const USDT_ID: u32 = 1984;
const INITIAL_BALANCE: u128 = 1_000_000;

fn create_usdt_and_mint_usdt_balance(beneficiary: <Test as frame_system::Config>::AccountId) {
	if !<Test as pallet_insurances::Config>::StableCurrency::asset_exists(USDT_ID) {
		assert_ok!(<Test as pallet_insurances::Config>::StableCurrency::force_create(
			RuntimeOrigin::root(),
			USDT_ID,
			88,
			false,
			1
		));
		Balances::make_free_balance_be(&88, INITIAL_BALANCE);
		assert_ok!(Assets::touch(RuntimeOrigin::signed(88), USDT_ID.into()));
		assert_ok!(<Test as pallet_insurances::Config>::StableCurrency::mint(
			RuntimeOrigin::signed(88),
			USDT_ID.into(),
			88,
			INITIAL_BALANCE
		));
	}

	Balances::make_free_balance_be(&beneficiary, INITIAL_BALANCE);
	assert_ok!(<Test as pallet_insurances::Config>::StableCurrency::mint(
		RuntimeOrigin::signed(88),
		USDT_ID.into(),
		beneficiary,
		INITIAL_BALANCE
	));
}

fn setup_testing_environment() {
	let dao_account_id = Dao::pallet_account_id().unwrap(); // is always Some, provided in genesis config

	create_usdt_and_mint_usdt_balance(ALICE);
	create_usdt_and_mint_usdt_balance(BOB);
	create_usdt_and_mint_usdt_balance(dao_account_id);
}

#[test]
fn add_member_works() {
	new_test_ext().execute_with(|| {
		// Read pallet storage and assert an expected result.
		assert_eq!(Members::<Test, _>::get().len(), 0 as usize);
		// Root can add new dao member.
		assert_ok!(Dao::add_member(RuntimeOrigin::root(), ALICE));
		System::assert_last_event(RuntimeEvent::Dao(crate::Event::DaoMemberAdded {
			new_member: ALICE,
		}));
		// Read pallet storage and assert an expected result.
		assert_eq!(Members::<Test, _>::get().contains(&ALICE), true);
		assert_eq!(Members::<Test, _>::get().len(), 1 as usize);

		assert_ok!(Dao::add_member(RuntimeOrigin::root(), BOB));
		System::assert_last_event(RuntimeEvent::Dao(crate::Event::DaoMemberAdded {
			new_member: BOB,
		}));
		assert_eq!(Members::<Test, _>::get().contains(&BOB), true);
		assert_eq!(Members::<Test, _>::get().len(), 2 as usize);
	});
}

#[test]
fn remove_member_works() {
	new_test_ext().execute_with(|| {
		// Read pallet storage and assert an expected result.
		assert_eq!(Members::<Test, _>::get().len(), 0 as usize);
		// Root can add new dao member.
		assert_ok!(Dao::add_member(RuntimeOrigin::root(), ALICE));
		System::assert_last_event(RuntimeEvent::Dao(crate::Event::DaoMemberAdded {
			new_member: ALICE,
		}));
		// Read pallet storage and assert an expected result.
		assert_eq!(Members::<Test, _>::get().contains(&ALICE), true);
		assert_eq!(Members::<Test, _>::get().len(), 1 as usize);

		assert_ok!(Dao::add_member(RuntimeOrigin::root(), BOB));
		System::assert_last_event(RuntimeEvent::Dao(crate::Event::DaoMemberAdded {
			new_member: BOB,
		}));
		assert_eq!(Members::<Test, _>::get().contains(&BOB), true);
		assert_eq!(Members::<Test, _>::get().len(), 2 as usize);

		assert_ok!(Dao::remove_member(RuntimeOrigin::root(), BOB));
		System::assert_last_event(RuntimeEvent::Dao(crate::Event::DaoMemberRemoved {
			old_member: BOB,
		}));
		assert_eq!(Members::<Test, _>::get().contains(&BOB), false);
		assert_eq!(Members::<Test, _>::get().len(), 1 as usize);

		assert_ok!(Dao::remove_member(RuntimeOrigin::root(), ALICE));
		System::assert_last_event(RuntimeEvent::Dao(crate::Event::DaoMemberRemoved {
			old_member: ALICE,
		}));
		assert_eq!(Members::<Test, _>::get().len(), 0 as usize);
	});
}

#[test]
fn request_insurance_works() {
	new_test_ext().execute_with(|| {
		setup_testing_environment();

		assert_eq!(NextProposalIndex::<Test, _>::get(), 0);

		let metadata = InsuranceMetadata {
			name: InsuranceType::Cyclone,
			location: 1,
			creator: BOB,
			status: InsuranceStatus::Active,
			underwrite_amount: 10_000,
			premium_amount: 100,
			contract_link: BoundedVec::try_from(vec![]).unwrap(),
			starts_on: 10,
			ends_on: 20,
			smt_id: None,
		};

		let proposal =
			RuntimeCall::Dao(crate::Call::allocate_liquidity { metadata: metadata.clone() });
		let proposal_hash = <Test as frame_system::Config>::Hashing::hash_of(&proposal);

		assert_ok!(Dao::request_insurance(
			RuntimeOrigin::signed(BOB),
			metadata,
			Box::new(proposal)
		));
		frame_system::Pallet::<Test>::set_block_number(5);
		<crate::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);
		System::assert_has_event(RuntimeEvent::Collective(pallet_collective::Event::Disapproved {
			proposal_hash,
		}));
		assert_eq!(crate::PendingProposalInfo::<Test, _>::iter().count(), 0);
	});
}

#[test]
fn request_insurance_fails_if_not_enough_funds() {
	new_test_ext().execute_with(|| {
		assert_eq!(NextProposalIndex::<Test, _>::get(), 0);

		let metadata = InsuranceMetadata {
			name: InsuranceType::Cyclone,
			location: 1,
			creator: BOB,
			status: InsuranceStatus::Active,
			underwrite_amount: 10_000,
			premium_amount: 100,
			contract_link: BoundedVec::try_from(vec![]).unwrap(),
			starts_on: 10,
			ends_on: 20,
			smt_id: None,
		};

		let proposal =
			RuntimeCall::Dao(crate::Call::allocate_liquidity { metadata: metadata.clone() });

		assert_noop!(
			Dao::request_insurance(RuntimeOrigin::signed(BOB), metadata, Box::new(proposal)),
			crate::Error::<Test, _>::NotEnoughFunds
		);
	});
}

#[test]
fn vote_fails_if_signer_is_not_a_member() {
	new_test_ext().execute_with(|| {
		setup_testing_environment();

		assert_ok!(Dao::add_member(RuntimeOrigin::root(), ALICE));

		let owner = RuntimeOrigin::signed(BOB);

		assert_eq!(NextProposalIndex::<Test, _>::get(), 0);

		let metadata = InsuranceMetadata {
			name: InsuranceType::Cyclone,
			location: 1,
			creator: BOB,
			status: InsuranceStatus::Active,
			underwrite_amount: 10_000,
			premium_amount: 100,
			contract_link: BoundedVec::try_from(vec![]).unwrap(),
			starts_on: 10,
			ends_on: 20,
			smt_id: None,
		};

		let proposal =
			RuntimeCall::Dao(crate::Call::allocate_liquidity { metadata: metadata.clone() });

		assert_ok!(Dao::request_insurance(owner.clone(), metadata, Box::new(proposal.clone())));

		let proposal_hash = <Test as frame_system::Config>::Hashing::hash_of(&proposal);

		assert_noop!(
			Dao::vote(RuntimeOrigin::signed(BOB), proposal_hash, 0, true),
			NotDaoMember::<Test, _>
		);
	});
}

#[test]
fn vote_works() {
	new_test_ext().execute_with(|| {
		setup_testing_environment();

		assert_ok!(Dao::add_member(RuntimeOrigin::root(), ALICE));

		let owner = RuntimeOrigin::signed(BOB);

		assert_eq!(NextProposalIndex::<Test, _>::get(), 0);

		let metadata = InsuranceMetadata {
			name: InsuranceType::Cyclone,
			location: 1,
			creator: BOB,
			status: InsuranceStatus::Active,
			underwrite_amount: 10_000,
			premium_amount: 100,
			contract_link: BoundedVec::try_from(vec![]).unwrap(),
			starts_on: 10,
			ends_on: 50,
			smt_id: None,
		};

		let proposal =
			RuntimeCall::Dao(crate::Call::allocate_liquidity { metadata: metadata.clone() });

		assert_ok!(Dao::request_insurance(owner.clone(), metadata, Box::new(proposal.clone())));

		let proposal_hash = <Test as frame_system::Config>::Hashing::hash_of(&proposal);

		frame_system::Pallet::<Test>::set_block_number(1);
		assert_ok!(Dao::vote(RuntimeOrigin::signed(ALICE), proposal_hash, 0, true));

		System::assert_has_event(RuntimeEvent::Dao(crate::Event::LiquidityProvisionVoted {
			who: ALICE,
			proposal_index: 0,
			decision: true,
		}));

		frame_system::Pallet::<Test>::set_block_number(5);
		<crate::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);
		System::assert_has_event(RuntimeEvent::Collective(pallet_collective::Event::Executed {
			result: Ok(()),
			proposal_hash,
		}));
		assert_eq!(crate::PendingProposalInfo::<Test, _>::iter().count(), 0);
	});
}

#[test]
fn user_tokens_unreserved_if_insurance_request_disapproved() {
	new_test_ext().execute_with(|| {
		setup_testing_environment();

		assert_eq!(NextProposalIndex::<Test, _>::get(), 0);

		let user_id = BOB;
		let premium_amount = 1_000;
		let underwrite_amount = 10_000;

		let metadata = InsuranceMetadata {
			name: InsuranceType::Cyclone,
			location: 1,
			creator: user_id,
			status: InsuranceStatus::Active,
			underwrite_amount,
			premium_amount,
			contract_link: BoundedVec::try_from(vec![]).unwrap(),
			starts_on: 10,
			ends_on: 20,
			smt_id: None,
		};

		let proposal =
			RuntimeCall::Dao(crate::Call::allocate_liquidity { metadata: metadata.clone() });

		let proposal_hash = <Test as frame_system::Config>::Hashing::hash_of(&proposal);

		assert_ok!(Dao::request_insurance(
			RuntimeOrigin::signed(user_id),
			metadata.clone(),
			Box::new(proposal)
		));
		System::assert_last_event(RuntimeEvent::Dao(crate::Event::InsuranceRequested {
			metadata: metadata.clone(),
			proposal_index: 0,
			proposal_hash,
		}));
		assert_eq!(
			<Test as pallet_insurances::Config>::StableCurrency::balance(USDT_ID, &user_id),
			INITIAL_BALANCE - premium_amount
		);

		frame_system::Pallet::<Test>::set_block_number(50);
		<crate::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);

		// DAO didn't vote on the proposal, and the voting period has ended, so the proposal got
		// rejected
		System::assert_has_event(RuntimeEvent::Collective(pallet_collective::Event::Disapproved {
			proposal_hash,
		}));

		assert_eq!(
			<Test as pallet_insurances::Config>::StableCurrency::balance(USDT_ID, &user_id),
			INITIAL_BALANCE
		);
	});
}

#[test]
fn dao_tokens_reserved_if_insurance_request_approved() {
	new_test_ext().execute_with(|| {
		setup_testing_environment();

		assert_eq!(NextProposalIndex::<Test, _>::get(), 0);

		let user_id = BOB;
		let dao_account_id = Dao::pallet_account_id().unwrap();
		assert_ok!(Dao::add_member(RuntimeOrigin::root(), ALICE));

		let underwrite_amount = 10_000;
		let premium_amount = 1_000;

		let metadata = InsuranceMetadata {
			name: InsuranceType::Cyclone,
			location: 1,
			creator: user_id,
			status: InsuranceStatus::Active,
			underwrite_amount,
			premium_amount: 1_000,
			contract_link: BoundedVec::try_from(vec![]).unwrap(),
			starts_on: 10,
			ends_on: 20,
			smt_id: None,
		};

		let proposal =
			RuntimeCall::Dao(crate::Call::allocate_liquidity { metadata: metadata.clone() });
		let proposal_hash = <Test as frame_system::Config>::Hashing::hash_of(&proposal);

		assert_ok!(Dao::request_insurance(
			RuntimeOrigin::signed(user_id),
			metadata.clone(),
			Box::new(proposal)
		));
		System::assert_last_event(RuntimeEvent::Dao(crate::Event::InsuranceRequested {
			metadata: metadata.clone(),
			proposal_hash,
			proposal_index: 0,
		}));

		assert_eq!(
			<Test as pallet_insurances::Config>::StableCurrency::balance(USDT_ID.into(), &user_id),
			INITIAL_BALANCE - premium_amount
		);

		frame_system::Pallet::<Test>::set_block_number(1);

		assert_ok!(Dao::vote(RuntimeOrigin::signed(ALICE), proposal_hash, 0, true));

		frame_system::Pallet::<Test>::set_block_number(5);
		<crate::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);
		System::assert_has_event(RuntimeEvent::Collective(pallet_collective::Event::Executed {
			result: Ok(()),
			proposal_hash,
		}));
		assert_eq!(
			<Test as pallet_insurances::Config>::StableCurrency::balance(USDT_ID.into(), &user_id),
			INITIAL_BALANCE - 1_000
		);
		assert_eq!(
			<Test as pallet_insurances::Config>::StableCurrency::balance(
				USDT_ID.into(),
				&dao_account_id
			),
			INITIAL_BALANCE - underwrite_amount
		);
	});
}
