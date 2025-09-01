//! Benchmarking setup for pallet-insurances

use super::*;
use pallet_insurances::types::{InsuranceMetadata, InsuranceStatus, InsuranceType};

use pallet_assets::{BenchmarkHelper, Pallet as Assets};

use frame_benchmarking::{account, benchmarks_instance_pallet, whitelisted_caller};
use frame_support::{
	assert_ok,
	sp_runtime::traits::Hash,
	traits::{
		fungibles::{Inspect, Mutate},
		Currency, Get, Hooks, OriginTrait, SortedMembers,
	},
};
use frame_system::RawOrigin;

use sp_runtime::{traits::StaticLookup, DispatchResult};
use sp_std::boxed::Box;

use num_traits::Bounded;

const USDT_ID: u32 = 1984;

fn assert_event_happened<T: Config<I>, I: 'static>(generic_event: <T as Config<I>>::RuntimeEvent) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::RuntimeEvent = generic_event.into();
	assert!(events.iter().find(|x| x.event == system_event).is_some());
}

fn create_usdt_and_mint_usdt_balance<T: Config<I>, I: 'static>(
	beneficiary: <T as frame_system::Config>::AccountId,
) -> DispatchResult {
	let dao_account_id = pallet_dao::Pallet::<T, I>::pallet_account_id().unwrap();
	let caller_lookup = <T as frame_system::Config>::Lookup::unlookup(dao_account_id.clone());
	let usdt_balance = <T as pallet_assets::Config>::Balance::max_value() / 1_000_u32.into();
	let usdt_id = <T as pallet_assets::Config>::BenchmarkHelper::create_asset_id_parameter(USDT_ID);

	if !Assets::<T>::asset_exists(usdt_id.into()) {
		Assets::<T>::force_create(
			RawOrigin::Root.into(),
			usdt_id,
			caller_lookup,
			true,
			1_u32.into(),
		)?;
		<T as Config<I>>::Currency::make_free_balance_be(
			&dao_account_id,
			<T as pallet_insurances::Config>::Balance::max_value() / 100_000u32.into(),
		);

		Assets::<T>::mint_into(usdt_id.into(), &dao_account_id, usdt_balance)?;
	}

	<T as Config<I>>::Currency::make_free_balance_be(
		&beneficiary,
		<T as pallet_insurances::Config>::Balance::max_value() / 100_000u32.into(),
	);
	Assets::<T>::mint_into(
		<T as pallet_assets::Config>::BenchmarkHelper::create_asset_id_parameter(USDT_ID).into(),
		&beneficiary,
		usdt_balance,
	)?;

	Ok(())
}

benchmarks_instance_pallet! {
	where_clause {
		where
			T: Config<I, OracleKey = (InsuranceType, u8), OracleValue = Option<(<T as pallet_insurances::Config>::NftId, <T as pallet_insurances::Config>::NftId)>>,
			T: pallet_collective::Config<I>,
			T: pallet_balances::Config,
			T: pallet_marketplace::Config<I>,
			<T as pallet_insurances::Config>::AssetId: From<u32>,
			<T as pallet_insurances::Config>::Balance: From<u32> + From<u128>,
			<T as frame_system::Config>::BlockNumber: From<u32>,
			<T as frame_system::Config>::RuntimeOrigin: OriginTrait<AccountId = <T as frame_system::Config>::AccountId>,
			<T as pallet_collective::Config<I>>::Proposal: From<pallet_dao::Call<T, I>>,
			<T as frame_system::Config>::RuntimeEvent: TryInto<pallet_collective::Event<T, I>>,
	}

	feed_event {
		let x in 0 .. <T as pallet_collective::Config<I>>::MaxProposals::get();
		let alice: <T as frame_system::Config>::AccountId = whitelisted_caller();
		let feeder: <T as frame_system::Config>::AccountId = <T as orml_oracle::Config<I>>::Members::sorted_members()[1].clone();
		let dao = pallet_dao::PalletAccountId::<T, I>::get().unwrap();
		assert_ok!(pallet_dao::Pallet::<T, I>::add_member(
			<T as frame_system::Config>::RuntimeOrigin::root(),
			alice.clone())
		);

		create_usdt_and_mint_usdt_balance::<T, I>(dao.clone())?;
		create_usdt_and_mint_usdt_balance::<T, I>(alice.clone())?;
		create_usdt_and_mint_usdt_balance::<T, I>(feeder.clone())?;

		let premium_amount = 10u128;

		for i in 0..x {
			let beneficiary: T::AccountId = account("beneficiary", i, 0);

			create_usdt_and_mint_usdt_balance::<T, I>(beneficiary.clone())?;

			let metadata: pallet_insurances::InsuranceMetadataOf<T> = InsuranceMetadata {
				name: InsuranceType::Cyclone,
				location: 1,
				creator: beneficiary.clone(),
				status: InsuranceStatus::Active,
				underwrite_amount: 1000u128.into(),
				premium_amount: premium_amount.into(),
				contract_link: Default::default(),
				starts_on: 10u32.into(),
				ends_on: 20u32.into(),
				smt_id: None,
			};

			let proposal: <T as pallet_collective::Config<I>>::Proposal = pallet_dao::Call::<T, I>::allocate_liquidity {
				metadata: metadata.clone(),
			}.into();
			let proposal_hash = <T as frame_system::Config>::Hashing::hash_of(&proposal);

			assert_ok!(pallet_dao::Pallet::<T, I>::request_insurance(
				<T as frame_system::Config>::RuntimeOrigin::signed(beneficiary),
				metadata.clone(),
				Box::new(proposal.into())
			));

			pallet_dao::Pallet::<T, I>::vote(
				<T as frame_system::Config>::RuntimeOrigin::signed(alice.clone()),
				proposal_hash,
				i,
				true
			).unwrap();
		}
		frame_system::Pallet::<T>::set_block_number(3.into());
		<pallet_dao::Pallet<T, I> as Hooks<<T as frame_system::Config>::BlockNumber>>::on_finalize(
			frame_system::Pallet::<T>::block_number()
		);
		frame_system::Pallet::<T>::set_block_number(10.into());
		<Pallet<T, I> as Hooks<<T as frame_system::Config>::BlockNumber>>::on_finalize(
			frame_system::Pallet::<T>::block_number()
		);
	}: feed_event(RawOrigin::Signed(feeder), (InsuranceType::Cyclone, 1_u8), sp_std::vec![])
	verify {
		assert_event_happened::<T, I>(Event::<T, I>::HandledInsuranceEvent {
			who: <T as orml_oracle::Config<I>>::Members::sorted_members()[1].clone(),
			event: InsuranceType::Cyclone,
			location: 1_u8,
			insurance: None,
		}.into());
	}

	claim_premium_payout {
		let caller: T::AccountId = whitelisted_caller();
		let dao = pallet_dao::PalletAccountId::<T, I>::get().unwrap();

		assert_ok!(pallet_dao::Pallet::<T, I>::add_member(
			<T as frame_system::Config>::RuntimeOrigin::root(),
			caller.clone())
		);
		let premium_amount: u128 = 10_000_000_000_000u128;
		let starts_on = 300u32;
		let ends_on = 400u32;
		let underwrite_amount: u128 = premium_amount * 10;

		create_usdt_and_mint_usdt_balance::<T, I>(dao.clone())?;
		create_usdt_and_mint_usdt_balance::<T, I>(caller.clone())?;

		let metadata: pallet_insurances::InsuranceMetadataOf<T> = InsuranceMetadata {
			name: InsuranceType::Cyclone,
			location: 1,
			creator: caller.clone(),
			status: InsuranceStatus::Active,
			underwrite_amount: underwrite_amount.into(),
			premium_amount: premium_amount.into(),
			contract_link: Default::default(),
			starts_on: starts_on.into(),
			ends_on: ends_on.into(),
			smt_id: None,
		};

		let proposal: <T as pallet_collective::Config<I>>::Proposal = pallet_dao::Call::<T, I>::allocate_liquidity {
			metadata: metadata.clone(),
		}.into();
		let proposal_hash = <T as frame_system::Config>::Hashing::hash_of(&proposal);

		let dao = pallet_dao::PalletAccountId::<T, I>::get().unwrap();
		assert_ok!(pallet_dao::Pallet::<T, I>::request_insurance(
			<T as frame_system::Config>::RuntimeOrigin::signed(caller.clone()),
			metadata.clone(),
			Box::new(proposal.into())
		));

		pallet_dao::Pallet::<T, I>::vote(
			<T as frame_system::Config>::RuntimeOrigin::signed(caller.clone()),
			proposal_hash,
			0,
			true
		).unwrap();

		frame_system::Pallet::<T>::set_block_number(200.into());
		<pallet_dao::Pallet<T, I> as Hooks<<T as frame_system::Config>::BlockNumber>>::on_finalize(
			frame_system::Pallet::<T>::block_number()
		);

		let collection_id = 0u32;
		let insurance_id = 0u32;
		assert_ok!(pallet_marketplace::Pallet::<T, I>::provide_liquidity(
			RawOrigin::Signed(caller.clone()).into(),
			collection_id.into(),
			insurance_id.into(),
		));
		frame_system::Pallet::<T>::set_block_number(ends_on.into());
		<Pallet<T, I> as Hooks<<T as frame_system::Config>::BlockNumber>>::on_finalize(
			frame_system::Pallet::<T>::block_number(),
		);
		let token_id = 0;
	}: _(RawOrigin::Signed(caller), token_id.into())

	do_execute_insurance_payout {
		frame_system::Pallet::<T>::set_block_number(1u32.into());
		let caller: T::AccountId = whitelisted_caller();
		let dao = pallet_dao::PalletAccountId::<T, I>::get().unwrap();

		assert_ok!(pallet_dao::Pallet::<T, I>::add_member(
			<T as frame_system::Config>::RuntimeOrigin::root(),
			caller.clone())
		);
		let premium_amount = 100u128;
		let starts_on = 100u32;
		let ends_on = 200u32;
		let underwrite_amount: u128 = premium_amount * 10;

		create_usdt_and_mint_usdt_balance::<T, I>(dao.clone())?;
		create_usdt_and_mint_usdt_balance::<T, I>(caller.clone())?;

		let metadata: pallet_insurances::InsuranceMetadataOf<T> = InsuranceMetadata {
			name: InsuranceType::Cyclone,
			location: 1,
			creator: caller.clone(),
			status: InsuranceStatus::Active,
			underwrite_amount: underwrite_amount.into(),
			premium_amount: premium_amount.into(),
			contract_link: Default::default(),
			starts_on: starts_on.into(),
			ends_on: ends_on.into(),
			smt_id: None,
		};

		let proposal: <T as pallet_collective::Config<I>>::Proposal = pallet_dao::Call::<T, I>::allocate_liquidity {
			metadata: metadata.clone(),
		}.into();
		let proposal_hash = <T as frame_system::Config>::Hashing::hash_of(&proposal);

		let dao = pallet_dao::PalletAccountId::<T, I>::get().unwrap();
		assert_ok!(pallet_dao::Pallet::<T, I>::request_insurance(
			<T as frame_system::Config>::RuntimeOrigin::signed(caller.clone()),
			metadata.clone(),
			Box::new(proposal.into())
		));

		pallet_dao::Pallet::<T, I>::vote(
			<T as frame_system::Config>::RuntimeOrigin::signed(caller.clone()),
			proposal_hash,
			0,
			true
		).unwrap();

		frame_system::Pallet::<T>::set_block_number(25u32.into());
		<pallet_dao::Pallet<T, I> as Hooks<<T as frame_system::Config>::BlockNumber>>::on_finalize(
			frame_system::Pallet::<T>::block_number()
		);

		let collection_id = 0u32;
		let insurance_id = 0u32;
		use frame_support::traits::Currency;
		<T as pallet::Config<I>>::Currency::make_free_balance_be(&caller, (underwrite_amount * 2).into());
		assert_ok!(pallet_marketplace::Pallet::<T, I>::provide_liquidity(
			RawOrigin::Signed(caller.clone()).into(),
			collection_id.into(),
			insurance_id.into(),
		));
		frame_system::Pallet::<T>::set_block_number(201u32.into());
	}: {
		Pallet::<T, I>::do_execute_insurance_payout(201u32.into())
	}
	verify {
		let _ = Pallet::<T, I>::insurance_count() == 0;
	}
}
