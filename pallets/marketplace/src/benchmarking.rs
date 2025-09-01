//! Benchmarking setup for pallet-template

use super::*;
use crate::types::OrderType;

use frame_benchmarking::{account, benchmarks_instance_pallet, whitelisted_caller};
use frame_support::traits::{
	fungibles::{Inspect, Mutate},
	Currency,
};
use frame_system::RawOrigin;
use num_traits::Bounded;
use sp_runtime::traits::StaticLookup;

use pallet_insurances::{types, InsuranceMetadataOf, Pallet as Insurances};

use pallet_assets::{BenchmarkHelper, Pallet as Assets};

const USDT_ID: u32 = 1984;

fn create_usdt_and_mint_usdt_balance<T: Config<I>, I: 'static>(
	beneficiary: <T as frame_system::Config>::AccountId,
) {
	let dao_account_id = pallet_dao::Pallet::<T, I>::pallet_account_id().unwrap();
	let caller_lookup = <T as frame_system::Config>::Lookup::unlookup(dao_account_id.clone());
	let usdt_balance = <T as pallet_assets::Config>::Balance::max_value() / 4_u32.into();

	if !<T as pallet_insurances::Config>::StableCurrency::asset_exists(USDT_ID.into()) {
		let _ = Assets::<T>::force_create(
			RawOrigin::Root.into(),
			<T as pallet_assets::Config>::BenchmarkHelper::create_asset_id_parameter(USDT_ID),
			caller_lookup,
			true,
			1_u32.into(),
		);
		<T as Config<I>>::Currency::make_free_balance_be(
			&dao_account_id,
			<T as pallet_insurances::Config>::Balance::max_value() / 4_u32.into(),
		);

		let _ = Assets::<T>::mint_into(
			<T as pallet_assets::Config>::BenchmarkHelper::create_asset_id_parameter(USDT_ID)
				.into(),
			&dao_account_id,
			usdt_balance,
		);
	}

	let _ = Assets::<T>::mint_into(
		<T as pallet_assets::Config>::BenchmarkHelper::create_asset_id_parameter(USDT_ID).into(),
		&beneficiary,
		usdt_balance,
	);
}

benchmarks_instance_pallet! {
	provide_liquidity {
		use num_traits::Zero;

		let collection_id: T::NftId = T::NftId::zero();
		let item_id: T::NftId = T::NftId::zero();
		let account: T::AccountId = whitelisted_caller();
		create_usdt_and_mint_usdt_balance::<T, I>(account.clone());

		let metadata: InsuranceMetadataOf<T> = types::InsuranceMetadata {
			name: types::InsuranceType::Cyclone,
			location: 0,
			creator: account.clone(),
			status: types::InsuranceStatus::NotStarted,
			underwrite_amount: <T as pallet_insurances::Config>::Balance::from(0_u32),
			premium_amount: <T as pallet_insurances::Config>::Balance::from(1_000u32),
			contract_link: frame_support::BoundedVec::with_max_capacity(),
			starts_on: T::BlockNumber::from(100u32),
			ends_on: T::BlockNumber::from(100_000u32),
			smt_id: None,
		};

		Insurances::<T>::do_mint_insured_nft(account.clone(), metadata)?;

		Insurances::<T>::do_mint_secondary_market_tokens(account.clone(), <T as pallet_insurances::Config>::Balance::from(100u32))?;
	}: _(RawOrigin::Signed(account), collection_id, item_id)

	create_order_buy {
		use num_traits::{Zero, Bounded};
		use frame_support::traits::Currency;

		let account: T::AccountId = whitelisted_caller();
		create_usdt_and_mint_usdt_balance::<T, I>(account.clone());

		Insurances::<T>::do_mint_secondary_market_tokens(account.clone(), <T as pallet_insurances::Config>::Balance::from(100u32))?;

		<T as Config<I>>::Currency::make_free_balance_be(&account, <T as pallet_insurances::Config>::Balance::max_value());
		assert_eq!(<T as Config<I>>::Currency::free_balance(&account), <T as pallet_insurances::Config>::Balance::max_value());

		let token_id = <T as pallet_insurances::Config>::AssetId::zero();
		let token_amount = <T as pallet_insurances::Config>::Balance::from(1u32);
		let price_per_token = <T as pallet_insurances::Config>::Balance::from(10u32);
		let order_type = OrderType::Buy;
	}: create_order(RawOrigin::Signed(account), token_id, token_amount, price_per_token, order_type)

	create_order_sell {
		let account: T::AccountId = whitelisted_caller();
		create_usdt_and_mint_usdt_balance::<T, I>(account.clone());

		Insurances::<T>::do_mint_secondary_market_tokens(account.clone(), <T as pallet_insurances::Config>::Balance::from(100u32))?;
		use num_traits::Zero;
		let token_id = <T as pallet_insurances::Config>::AssetId::zero();
		let token_amount = <T as pallet_insurances::Config>::Balance::from(10u32);
		let price_per_token = <T as pallet_insurances::Config>::Balance::from(100u32);
		let order_type = OrderType::Sell;
	}: create_order(RawOrigin::Signed(account), token_id, token_amount, price_per_token, order_type)

	cancel_order_buy {
		use num_traits::Zero;

		let account: T::AccountId = whitelisted_caller();
		create_usdt_and_mint_usdt_balance::<T, I>(account.clone());

		let token_id = <T as pallet_insurances::Config>::AssetId::zero();
		let token_amount = <T as pallet_insurances::Config>::Balance::from(1u32);
		let price_per_token = <T as pallet_insurances::Config>::Balance::from(100u32);
		let order_type = OrderType::Buy;
		Pallet::<T, I>::create_order(RawOrigin::Signed(account.clone()).into(), token_id, token_amount, price_per_token, order_type).unwrap();
		let order_id = T::OrderId::zero();
	}: cancel_order(RawOrigin::Signed(account), order_id)

	cancel_order_sell {
		let account: T::AccountId = whitelisted_caller();
		create_usdt_and_mint_usdt_balance::<T, I>(account.clone());

		Insurances::<T>::do_mint_secondary_market_tokens(account.clone(), <T as pallet_insurances::Config>::Balance::from(100u32))?;
		use num_traits::Zero;
		let token_id = <T as pallet_insurances::Config>::AssetId::zero();
		let token_amount = <T as pallet_insurances::Config>::Balance::from(10u32);
		let price_per_token = <T as pallet_insurances::Config>::Balance::from(100u32);
		let order_type = OrderType::Sell;
		Pallet::<T, I>::create_order(RawOrigin::Signed(account.clone()).into(), token_id, token_amount, price_per_token, order_type).unwrap();
		let order_id = T::OrderId::zero();
	}: cancel_order(RawOrigin::Signed(account), order_id)

	fulfill_order_buy {
		use num_traits::Zero;

		let creator: T::AccountId = whitelisted_caller();
		let token_id = <T as pallet_insurances::Config>::AssetId::zero();
		let token_amount = <T as pallet_insurances::Config>::Balance::from(10u32);
		let price_per_token = <T as pallet_insurances::Config>::Balance::from(10u32);
		let order_type = OrderType::Buy;

		let fulfiller: T::AccountId = account("fulfiller", 0, 0);
		create_usdt_and_mint_usdt_balance::<T, I>(fulfiller.clone());
		create_usdt_and_mint_usdt_balance::<T, I>(creator.clone());

		Insurances::<T>::do_mint_secondary_market_tokens(fulfiller.clone(), 10u32.into())?;

		Pallet::<T, I>::create_order(RawOrigin::Signed(creator.clone()).into(), token_id, token_amount, price_per_token, order_type).unwrap();
		let order_id = T::OrderId::zero();
	}: fulfill_order(RawOrigin::Signed(fulfiller), order_id, token_amount - 1u32.into())

	fulfill_order_sell {
		use num_traits::{Zero, Bounded};
		let creator: T::AccountId = whitelisted_caller();
		let fulfiller: T::AccountId = account("fulfiller", 0, 0);

		create_usdt_and_mint_usdt_balance::<T, I>(creator.clone());
		create_usdt_and_mint_usdt_balance::<T, I>(fulfiller.clone());

		Insurances::<T>::do_mint_secondary_market_tokens(creator.clone(), <T as pallet_insurances::Config>::Balance::from(10u32))?;
		let token_id = <T as pallet_insurances::Config>::AssetId::zero();
		let token_amount = <T as pallet_insurances::Config>::Balance::from(10u32);
		let price_per_token = <T as pallet_insurances::Config>::Balance::from(10u32);
		let order_type = OrderType::Sell;

		use frame_support::traits::Currency;
		<T as Config<I>>::Currency::make_free_balance_be(&fulfiller, <T as pallet_insurances::Config>::Balance::max_value());

		Pallet::<T, I>::create_order(RawOrigin::Signed(creator.clone()).into(), token_id, token_amount, price_per_token, order_type).unwrap();
		let order_id = T::OrderId::zero();
	}: fulfill_order(RawOrigin::Signed(fulfiller), order_id, token_amount - 1u32.into())
}
