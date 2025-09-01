use super::NextSmtId;
use crate::{
	mock::*,
	types::{InsuranceMetadata, InsuranceStatus, InsuranceType},
	Error,
};
use frame_support::{assert_err, assert_ok, BoundedVec};

#[test]
fn mint_asset_works() {
	new_test_ext().execute_with(|| {
		// Root can mint 100 assets to any account
		assert_ok!(Insurances::do_mint_secondary_market_tokens(ALICE, 100));
		System::assert_last_event(RuntimeEvent::Insurances(
			crate::Event::SecondaryMarketTokensMinted { owner: ALICE, asset_id: 0, amount: 100 },
		));
		assert_eq!(Assets::balance(0, ALICE), 100);
		assert_eq!(NextSmtId::<Test>::get(), 1);

		// Asset id autoincrements
		assert_ok!(Insurances::do_mint_secondary_market_tokens(BOB, 100));
		System::assert_last_event(RuntimeEvent::Insurances(
			crate::Event::SecondaryMarketTokensMinted { owner: BOB, asset_id: 1, amount: 100 },
		));
		assert_eq!(Assets::balance(1, BOB), 100);
		assert_eq!(NextSmtId::<Test>::get(), 2);

		NextSmtId::<Test>::mutate(|id| *id = 0);
		assert_err!(
			Insurances::do_mint_secondary_market_tokens(BOB, 100),
			pallet_assets::Error::<Test>::InUse
		);

		// AssetId exhaustion causes an error
		NextSmtId::<Test>::mutate(|id| *id = u64::max_value());
		assert_err!(
			Insurances::do_mint_secondary_market_tokens(ALICE, 100),
			Error::<Test>::NoAvailableAssetId
		);

		NextSmtId::<Test>::mutate(|id| *id = 2);
	});
}

#[test]
fn mint_insured_nft_work() {
	new_test_ext().execute_with(|| {
		use frame_support::traits::Currency;

		Balances::make_free_balance_be(&ALICE, 1_000_000);

		let metadata: crate::InsuranceMetadataOf<Test> = InsuranceMetadata {
			name: InsuranceType::Cyclone,
			location: 0,
			creator: ALICE,
			status: InsuranceStatus::NotStarted,
			underwrite_amount: <Test as crate::Config>::Balance::from(1_000_000u32),
			premium_amount: <Test as crate::Config>::Balance::from(1_000u32),
			contract_link: BoundedVec::with_max_capacity(),
			starts_on: <Test as frame_system::Config>::BlockNumber::from(100u32),
			ends_on: <Test as frame_system::Config>::BlockNumber::from(100_000u32),
			smt_id: None,
		};

		// We can request multiple insurances with the same metadata
		assert_ok!(Insurances::do_mint_insured_nft(ALICE, metadata.clone()));

		assert_ok!(Insurances::do_mint_insured_nft(ALICE, metadata.clone()));
	});
}

#[test]
fn test_next_user_collection() {
	new_test_ext().execute_with(|| {
		use frame_support::traits::{
			tokens::nonfungibles::{Create as _, InspectEnumerable as _, Mutate},
			Currency,
		};
		use num_traits::Zero;

		Balances::make_free_balance_be(&BOB, 1_000_000);
		for i in 0..10 {
			let (collection_id, item_id) =
				if <Test as crate::Config>::InsuredToken::owned(&BOB).nth(0).is_some() {
					let user_items = <Test as crate::Config>::InsuredToken::owned(&BOB);
					let mut vec_user_items = user_items.collect::<Vec<_>>();
					vec_user_items.sort();
					let (collection_id, item_id) = vec_user_items.last().unwrap().clone();
					assert_eq!(
						<Test as crate::Config>::InsuredToken::owned_in_collection(
							&collection_id,
							&BOB
						)
						.count(),
						i as usize
					);
					assert_eq!(i - 1, item_id);
					(collection_id, item_id + 1_u64)
				} else {
					let collection_id = Insurances::get_next_collection_id().unwrap();
					assert_ok!(<Test as crate::Config>::InsuredToken::create_collection(
						&collection_id,
						&BOB,
						&BOB
					));
					(collection_id, <Test as crate::Config>::AssetId::zero())
				};
			assert_eq!(item_id, i);
			assert_ok!(<Test as crate::Config>::InsuredToken::mint_into(
				&collection_id,
				&item_id,
				&BOB
			));
		}
	});
}
