//! Benchmarking setup for pallet-dao

use super::*;
use frame_benchmarking::{account, benchmarks_instance_pallet, whitelisted_caller};
use frame_support::{
	sp_runtime::traits::Hash,
	traits::{fungibles::Mutate, Currency},
};
use frame_system::{Call as SystemCall, RawOrigin};
use num_traits::Bounded;
use sp_core::Get;
use sp_runtime::traits::StaticLookup;
use sp_std::{boxed::Box, prelude::*, vec};

use crate::Pallet as Dao;
use pallet_assets::{BenchmarkHelper, Pallet as Assets};

use pallet_insurances::{types, InsuranceMetadataOf};

benchmarks_instance_pallet! {
	add_member {
		let new_member = account("new_account", 12, 0);
	}: _(RawOrigin::Root, new_member)

	remove_member {
		let dao_member = account::<T::AccountId>("new_account", 12 , 0);
		Pallet::<T, I>::add_member(RawOrigin::Root.into(), dao_member.clone())?;
	}: _(RawOrigin::Root, dao_member)

	request_insurance {
		let caller: T::AccountId = whitelisted_caller();
		let caller_lookup = <T as frame_system::Config>::Lookup::unlookup(caller.clone());
		let dao_account_id = Pallet::<T, I>::pallet_account_id().unwrap();

		let usdt_id = <T as pallet_insurances::Config>::UsdtId::get().into();
		let usdt_value = <T as pallet_assets::Config>::Balance::max_value() / 2_u32.into();
		let value = <T::LocalCurrency as Currency::<T::AccountId>>::Balance::max_value();

		T::LocalCurrency::make_free_balance_be(&caller.clone(), value);
		T::LocalCurrency::make_free_balance_be(&dao_account_id, value);

		let asset_id = Assets::<T>::force_create(
			RawOrigin::Root.into(),
			<T as pallet_assets::Config>::BenchmarkHelper::create_asset_id_parameter(usdt_id),
			caller_lookup,
			true,
			1_u32.into()
		);

		Assets::<T>::mint_into(
			<T as pallet_assets::Config>::BenchmarkHelper::create_asset_id_parameter(usdt_id).into(),
			&caller,
			usdt_value,
		)?;

		Assets::<T>::mint_into(
			<T as pallet_assets::Config>::BenchmarkHelper::create_asset_id_parameter(usdt_id).into(),
			&dao_account_id,
			usdt_value,
		)?;

		let metadata: InsuranceMetadataOf<T> = types::InsuranceMetadata {
			name: types::InsuranceType::Cyclone,
			location: 0,
			creator: caller.clone(),
			status: types::InsuranceStatus::NotStarted,
			underwrite_amount: <T as pallet_insurances::Config>::Balance::from(100u32),
			premium_amount: <T as pallet_insurances::Config>::Balance::from(1u32),
			contract_link: frame_support::BoundedVec::with_max_capacity(),
			starts_on: T::BlockNumber::from(100u32),
			ends_on: T::BlockNumber::from(100_000u32),
			smt_id: None,
		};

		let proposal = SystemCall::<T>::remark { remark: vec![1; MAX_BYTES as usize] }.into();

	}: _(RawOrigin::Signed(caller), metadata, Box::new(proposal))

	vote {
		let caller: T::AccountId = whitelisted_caller();
		let caller_lookup = <T as frame_system::Config>::Lookup::unlookup(caller.clone());
		let voter = account::<T::AccountId>("voter", 12, 0);
		let dao_account_id = Pallet::<T, I>::pallet_account_id().unwrap();

		let usdt_id = <T as pallet_insurances::Config>::UsdtId::get().into();
		let value = <T::LocalCurrency as Currency::<T::AccountId>>::Balance::max_value();
		let usdt_value = <T as pallet_assets::Config>::Balance::max_value() / 4_u32.into();

		T::LocalCurrency::make_free_balance_be(&caller.clone(), value);
		T::LocalCurrency::make_free_balance_be(&voter.clone(), value);
		T::LocalCurrency::make_free_balance_be(&dao_account_id, value);

		let asset_id = Assets::<T>::force_create(
			RawOrigin::Root.into(),
			<T as pallet_assets::Config>::BenchmarkHelper::create_asset_id_parameter(usdt_id),
			caller_lookup,
			true,
			1_u32.into()
		);

		Assets::<T>::mint_into(
			<T as pallet_assets::Config>::BenchmarkHelper::create_asset_id_parameter(usdt_id).into(),
			&caller,
			usdt_value,
		)?;

		Assets::<T>::mint_into(
			<T as pallet_assets::Config>::BenchmarkHelper::create_asset_id_parameter(usdt_id).into(),
			&dao_account_id,
			usdt_value
		)?;

		Assets::<T>::mint_into(
			<T as pallet_assets::Config>::BenchmarkHelper::create_asset_id_parameter(usdt_id).into(),
			&voter,
			usdt_value
		)?;

		let metadata: InsuranceMetadataOf<T> = types::InsuranceMetadata {
			name: types::InsuranceType::Cyclone,
			location: 0,
			creator: caller.clone(),
			status: types::InsuranceStatus::NotStarted,
			underwrite_amount: <T as pallet_insurances::Config>::Balance::from(1_000u32),
			premium_amount: <T as pallet_insurances::Config>::Balance::from(100u32),
			contract_link: frame_support::BoundedVec::with_max_capacity(),
			starts_on: T::BlockNumber::from(100u32),
			ends_on: T::BlockNumber::from(100_000u32),
			smt_id: None,
		};

		let caller_origin = <T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(caller.clone()));
		let proposal: <T as pallet_collective::Config<_>>::Proposal = SystemCall::<T>::remark { remark: vec![1; MAX_BYTES as usize] }.into();
		let proposal_hash = <T as frame_system::Config>::Hashing::hash_of(&proposal.clone());

		let result = Pallet::<T, I>::request_insurance(caller_origin.clone(), metadata, Box::new(proposal))?;
		Pallet::<T, I>::add_member(RawOrigin::Root.into(), voter.clone())?;

	}: _(RawOrigin::Signed(voter), proposal_hash, 0, true)

	allocate_liquidity {
		let caller: T::AccountId = whitelisted_caller();
		let caller_lookup = <T as frame_system::Config>::Lookup::unlookup(caller.clone());
		let voter = account::<T::AccountId>("voter", 12, 0);
		let dao_account_id = Pallet::<T, I>::pallet_account_id().unwrap();

		let usdt_id = <T as pallet_insurances::Config>::UsdtId::get().into();
		let value = <T::LocalCurrency as Currency::<T::AccountId>>::Balance::max_value();
		let usdt_value = <T as pallet_assets::Config>::Balance::max_value() / 4_u32.into();

		T::LocalCurrency::make_free_balance_be(&caller.clone(), value);
		T::LocalCurrency::make_free_balance_be(&voter.clone(), value);
		T::LocalCurrency::make_free_balance_be(&dao_account_id, value);

		let asset_id = Assets::<T>::force_create(
			RawOrigin::Root.into(),
			<T as pallet_assets::Config>::BenchmarkHelper::create_asset_id_parameter(usdt_id),
			caller_lookup,
			true,
			1_u32.into()
		);

		Assets::<T>::mint_into(
			<T as pallet_assets::Config>::BenchmarkHelper::create_asset_id_parameter(usdt_id).into(),
			&caller,
			usdt_value,
		)?;

		Assets::<T>::mint_into(
			<T as pallet_assets::Config>::BenchmarkHelper::create_asset_id_parameter(usdt_id).into(),
			&dao_account_id,
			usdt_value
		)?;

		Assets::<T>::mint_into(
			<T as pallet_assets::Config>::BenchmarkHelper::create_asset_id_parameter(usdt_id).into(),
			&voter,
			usdt_value
		)?;

		let metadata: InsuranceMetadataOf<T> = types::InsuranceMetadata {
			name: types::InsuranceType::Cyclone,
			location: 0,
			creator: caller.clone(),
			status: types::InsuranceStatus::NotStarted,
			underwrite_amount: <T as pallet_insurances::Config>::Balance::from(0_u32),
			premium_amount: <T as pallet_insurances::Config>::Balance::from(1_000u32),
			contract_link: frame_support::BoundedVec::with_max_capacity(),
			starts_on: T::BlockNumber::from(100u32),
			ends_on: T::BlockNumber::from(100_000u32),
			smt_id: None,
		};

		let caller_origin = <T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(caller.clone()));
		let proposal: <T as pallet_collective::Config<_>>::Proposal = SystemCall::<T>::remark { remark: vec![1; MAX_BYTES as usize] }.into();
		let proposal_hash = <T as frame_system::Config>::Hashing::hash_of(&proposal.clone());

		Pallet::<T, I>::request_insurance(caller_origin.clone(), metadata.clone(), Box::new(proposal))?;
		Pallet::<T, I>::add_member(RawOrigin::Root.into(), voter.clone())?;
		Pallet::<T, I>::vote(RawOrigin::Signed(voter.clone()).into(), proposal_hash, 0, true)?;
	}: allocate_liquidity(RawOrigin::Root, metadata)

	do_execute_voting_ended {
		frame_system::Pallet::<T>::set_block_number(1u32.into());
		let caller: T::AccountId = whitelisted_caller();
		let caller_lookup = <T as frame_system::Config>::Lookup::unlookup(caller.clone());
		let voter = account::<T::AccountId>("voter", 12, 0);
		let dao_account_id = Pallet::<T, I>::pallet_account_id().unwrap();

		let usdt_id = <T as pallet_insurances::Config>::UsdtId::get().into();
		let value = <T::LocalCurrency as Currency::<T::AccountId>>::Balance::max_value();
		let usdt_value = <T as pallet_assets::Config>::Balance::max_value() / 4_u32.into();

		T::LocalCurrency::make_free_balance_be(&caller.clone(), value);
		T::LocalCurrency::make_free_balance_be(&voter.clone(), value);
		T::LocalCurrency::make_free_balance_be(&dao_account_id, value);

		let asset_id = Assets::<T>::force_create(
			RawOrigin::Root.into(),
			<T as pallet_assets::Config>::BenchmarkHelper::create_asset_id_parameter(usdt_id),
			caller_lookup,
			true,
			1_u32.into()
		);

		Assets::<T>::mint_into(
			<T as pallet_assets::Config>::BenchmarkHelper::create_asset_id_parameter(usdt_id).into(),
			&caller,
			usdt_value,
		)?;

		Assets::<T>::mint_into(
			<T as pallet_assets::Config>::BenchmarkHelper::create_asset_id_parameter(usdt_id).into(),
			&dao_account_id,
			usdt_value
		)?;

		Assets::<T>::mint_into(
			<T as pallet_assets::Config>::BenchmarkHelper::create_asset_id_parameter(usdt_id).into(),
			&voter,
			usdt_value
		)?;

		let metadata: InsuranceMetadataOf<T> = types::InsuranceMetadata {
			name: types::InsuranceType::Cyclone,
			location: 0,
			creator: caller.clone(),
			status: types::InsuranceStatus::NotStarted,
			underwrite_amount: <T as pallet_insurances::Config>::Balance::from(0_u32),
			premium_amount: <T as pallet_insurances::Config>::Balance::from(1_000u32),
			contract_link: frame_support::BoundedVec::with_max_capacity(),
			starts_on: T::BlockNumber::from(5u32),
			ends_on: T::BlockNumber::from(10u32),
			smt_id: None,
		};

		let caller_origin = <T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(caller.clone()));
		let proposal: <T as pallet_collective::Config<_>>::Proposal = SystemCall::<T>::remark { remark: vec![1; MAX_BYTES as usize] }.into();
		let proposal_hash = <T as frame_system::Config>::Hashing::hash_of(&proposal.clone());

		Pallet::<T, I>::request_insurance(caller_origin.clone(), metadata.clone(), Box::new(proposal))?;
		Pallet::<T, I>::add_member(RawOrigin::Root.into(), voter.clone())?;
		Pallet::<T, I>::vote(RawOrigin::Signed(voter.clone()).into(), proposal_hash, 0, true)?;

		let voting_metadata = Dao::<T, I>::active_voting_metadata(proposal_hash).unwrap();

		frame_system::Pallet::<T>::set_block_number(25u32.into());
	}: {
		Dao::<T, I>::do_execute_voting_ended(25u32.into())
	}
	verify {
		assert!(Dao::<T, I>::active_voting_metadata(proposal_hash).is_none());
		assert_eq!(Pallet::<T, I>::pending_proposal_info((voting_metadata.beneficiary, voting_metadata.metadata_hash)), None);
	}
}

const MAX_BYTES: u32 = 36;

#[test]
fn test_benchmarks() {
	use crate::mock::{new_test_ext, Test};
	new_test_ext().execute_with(|| {
		frame_support::assert_ok!(Pallet::<Test>::test_benchmark_do_allocate_liquidity());
	});
}
