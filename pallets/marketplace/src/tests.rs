use crate::{self as pallet_marketplace, mock::*, types::OrderType, Error};
use frame_support::{
	assert_noop, assert_ok,
	traits::{
		fungibles::{Inspect, Unbalanced},
		tokens::{Fortitude, Precision, Preservation},
		Currency,
	},
};
use pallet_insurances::{
	types::{InsuranceMetadata, InsuranceStatus, InsuranceType},
	Metadata,
};
use sp_runtime::MultiAddress;

const ASSET_MINT_COUNT: u128 = 100;
const USDT_ID: u32 = 1984;
const INITIAL_BALANCE: u128 = 1_000_000;

fn create_usdt_and_mint_usdt_balance(beneficiary: <Test as frame_system::Config>::AccountId) {
	if !<Test as pallet_insurances::Config>::StableCurrency::asset_exists(USDT_ID) {
		assert_ok!(<Test as pallet_insurances::Config>::StableCurrency::force_create(
			RuntimeOrigin::root(),
			USDT_ID,
			MultiAddress::Id(10),
			false,
			1
		));
		Balances::make_free_balance_be(&10, INITIAL_BALANCE);
		assert_ok!(Assets::touch(RuntimeOrigin::signed(10), USDT_ID));
		assert_ok!(<Test as pallet_insurances::Config>::StableCurrency::mint(
			RuntimeOrigin::signed(10),
			USDT_ID,
			MultiAddress::Id(10),
			INITIAL_BALANCE
		));
	}

	Balances::make_free_balance_be(&beneficiary, INITIAL_BALANCE);
	assert_ok!(<Test as pallet_insurances::Config>::StableCurrency::mint(
		RuntimeOrigin::signed(10),
		USDT_ID,
		MultiAddress::Id(beneficiary),
		INITIAL_BALANCE
	));
}

fn setup_secondary_market_tokens_and_balance(
	beneficiary: <Test as frame_system::Config>::AccountId,
) -> <Test as pallet_assets::Config>::AssetId {
	assert_ok!(Insurances::do_mint_secondary_market_tokens(beneficiary, ASSET_MINT_COUNT));

	Balances::make_free_balance_be(&beneficiary, INITIAL_BALANCE);
	Balances::make_free_balance_be(&ALICE, INITIAL_BALANCE);

	let token_id = System::events()
		.iter()
		.rev()
		.find_map(|event| {
			if let RuntimeEvent::Insurances(
				pallet_insurances::Event::<Test>::SecondaryMarketTokensMinted { asset_id, .. },
			) = event.event
			{
				Some(asset_id)
			} else {
				None
			}
		})
		.unwrap();
	assert_eq!(
		<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(token_id, beneficiary),
		ASSET_MINT_COUNT
	);
	token_id
}

fn create_order(
	creator: <Test as frame_system::Config>::AccountId,
	token_id: <Test as pallet_insurances::Config>::AssetId,
	token_amount: <Test as pallet_insurances::Config>::Balance,
	price_per_token: <<Test as pallet_marketplace::Config<pallet_marketplace::Instance1>>::Currency as frame_support::traits::Currency<<Test as frame_system::Config>::AccountId>>::Balance,
	order_type: OrderType,
) -> <Test as pallet_marketplace::Config<pallet_marketplace::Instance1>>::OrderId {
	assert_ok!(Marketplace::create_order(
		RuntimeOrigin::signed(creator),
		token_id,
		token_amount,
		price_per_token,
		order_type
	));
	System::events()
		.iter()
		.rev()
		.find_map(|event| {
			if let RuntimeEvent::Marketplace(pallet_marketplace::Event::<
				Test,
				pallet_marketplace::Instance1,
			>::OrderCreated {
				id,
				..
			}) = event.event
			{
				Some(id)
			} else {
				None
			}
		})
		.unwrap()
}

fn setup_testing_environment() -> <Test as pallet_assets::Config>::AssetId {
	let dao_account_id = Dao::pallet_account_id().unwrap(); // is always Some, provided in genesis config

	create_usdt_and_mint_usdt_balance(ALICE);
	create_usdt_and_mint_usdt_balance(BOB);
	create_usdt_and_mint_usdt_balance(dao_account_id);

	setup_secondary_market_tokens_and_balance(ALICE)
}

macro_rules! total_balance {
	($who:ident) => {
		<Test as pallet_insurances::Config>::StableCurrency::reducible_balance(
			USDT_ID,
			&$who,
			Preservation::Preserve,
			Fortitude::Polite,
		)
	};
}

#[test]
fn provide_liquidity_works() {
	new_test_ext().execute_with(|| {
		setup_testing_environment();

		// 2. Test for invalid SecondaryMarketTokenId
		let not_minted_secondary_market_token = 1;
		assert_noop!(
			Marketplace::provide_liquidity(
				RuntimeOrigin::signed(ALICE),
				not_minted_secondary_market_token,
				not_minted_secondary_market_token,
			),
			crate::Error::<Test, pallet_marketplace::Instance1>::NoMetadataFound
		);

		let underwrite_amount = 100;
		let metadata = InsuranceMetadata {
			name: InsuranceType::Cyclone,
			location: 1,
			creator: ALICE,
			status: InsuranceStatus::Active,
			underwrite_amount,
			premium_amount: 10,
			contract_link: Default::default(),
			starts_on: 10,
			ends_on: 20,
			smt_id: None,
		};

		// 2.1 mint tokens for SMTId 1
		assert_eq!(setup_secondary_market_tokens_and_balance(ALICE), 1);

		// 3. valid test
		let insurance_id = 0;
		Metadata::<Test>::insert(insurance_id, insurance_id, metadata.clone());
		assert_ok!(Marketplace::provide_liquidity(
			RuntimeOrigin::signed(ALICE),
			insurance_id,
			insurance_id
		));

		// check for event
		System::assert_last_event(RuntimeEvent::Marketplace(crate::Event::LiquidityProvided {
			who: ALICE,
			smt_id: 2,
			collection_id: insurance_id,
			item_id: insurance_id,
		}));

		// can't buy the same insurance again
		let insurance_id = 0;
		assert_noop!(
			Marketplace::provide_liquidity(
				RuntimeOrigin::signed(ALICE),
				insurance_id,
				insurance_id
			),
			Error::<Test, pallet_marketplace::Instance1>::InsuranceAlreadySold
		);

		// test for invalid MetadataId (say, 3 or 88)
		let invalid_insurance_id = 88;
		assert_noop!(
			Marketplace::provide_liquidity(
				RuntimeOrigin::signed(ALICE),
				invalid_insurance_id,
				invalid_insurance_id
			),
			Error::<Test, pallet_marketplace::Instance1>::NoMetadataFound
		);

		// Assets are minted for ALICE, try to provide_liquidity for BOB
		// Check that it is valid to mint by one user and provide_liquidity to another
		let valid_insurance_id = 1;
		Metadata::<Test>::insert(valid_insurance_id, valid_insurance_id, metadata.clone());
		assert_ok!(Marketplace::provide_liquidity(
			RuntimeOrigin::signed(BOB),
			valid_insurance_id,
			valid_insurance_id
		));
	});
}

#[test]
fn create_order_works() {
	new_test_ext().execute_with(|| {
		let token_id = setup_testing_environment();

		create_order(ALICE, token_id, 10, 100, OrderType::Sell);
		// `token_amount` tokens are locked
		assert_eq!(
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(token_id, ALICE),
			ASSET_MINT_COUNT - 10
		);

		create_order(ALICE, token_id, 20, 100, OrderType::Buy);
		// `token amount * price_per_token` is reserved
		assert_eq!(
			<Test as pallet_insurances::Config>::StableCurrency::reducible_balance(
				USDT_ID,
				&ALICE,
				Preservation::Preserve,
				Fortitude::Polite,
			),
			INITIAL_BALANCE - 20 * 100 - 1
		);

		pallet_marketplace::OrderBook::<Test, pallet_marketplace::Instance1>::iter().for_each(
			|(key, value)| {
				System::assert_has_event(RuntimeEvent::Marketplace(pallet_marketplace::Event::<
					Test,
					pallet_marketplace::Instance1,
				>::OrderCreated {
					id: key,
					info: value.clone(),
				}));
			},
		);

		// can't sell tokens you do not possess
		assert_noop!(
			Marketplace::create_order(
				RuntimeOrigin::signed(BOB),
				token_id,
				10,
				100,
				OrderType::Sell
			),
			pallet_marketplace::Error::<Test, pallet_marketplace::Instance1>::InvalidTokensAmount
		);
	});
}

#[test]
fn cancel_order_works() {
	new_test_ext().execute_with(|| {
		let token_id = setup_testing_environment();

		let balance = <Test as pallet_insurances::Config>::StableCurrency::reducible_balance(
			USDT_ID,
			&ALICE,
			Preservation::Preserve,
			Fortitude::Polite,
		);
		let token_balance =
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(token_id, ALICE);

		let order_id = create_order(ALICE, token_id, 10, 100, OrderType::Sell);

		// can't cancel non-existent order
		assert_noop!(
			Marketplace::cancel_order(RuntimeOrigin::signed(ALICE), order_id + 1),
			pallet_marketplace::Error::<Test, pallet_marketplace::Instance1>::InvalidOrderId
		);
		// can't cancel not own order
		assert_noop!(
			Marketplace::cancel_order(RuntimeOrigin::signed(BOB), order_id),
			pallet_marketplace::Error::<Test, pallet_marketplace::Instance1>::OrderCreatorMismatch
		);
		assert_ok!(Marketplace::cancel_order(RuntimeOrigin::signed(ALICE), order_id));

		System::assert_has_event(RuntimeEvent::Marketplace(pallet_marketplace::Event::<
			Test,
			pallet_marketplace::Instance1,
		>::OrderCanceled {
			id: order_id,
		}));

		let order_id = create_order(ALICE, token_id, 10, 100, OrderType::Buy);
		assert_ok!(Marketplace::cancel_order(RuntimeOrigin::signed(ALICE), order_id));
		System::assert_has_event(RuntimeEvent::Marketplace(pallet_marketplace::Event::<
			Test,
			pallet_marketplace::Instance1,
		>::OrderCanceled {
			id: order_id,
		}));

		assert_eq!(
			<Test as pallet_insurances::Config>::StableCurrency::reducible_balance(
				USDT_ID,
				&ALICE,
				Preservation::Preserve,
				Fortitude::Polite,
			),
			balance
		);
		assert_eq!(
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(token_id, ALICE),
			token_balance
		);

		assert_eq!(
			pallet_marketplace::OrderBook::<Test, pallet_marketplace::Instance1>::iter().count(),
			0
		);
	});
}

#[test]
fn cancel_partially_fulfilled_order_works() {
	new_test_ext().execute_with(|| {
		let token_id = setup_testing_environment();

		let token_balance =
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(token_id, ALICE);
		let order_token_balance = 50;
		let order_id =
			create_order(ALICE, token_id, order_token_balance.clone(), 100, OrderType::Sell);
		let order_info =
			pallet_marketplace::OrderBook::<Test, pallet_marketplace::Instance1>::get(order_id)
				.unwrap();

		assert_eq!(
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(token_id, ALICE),
			token_balance - order_info.token_amount
		);

		// can't cancel non-existent order
		assert_noop!(
			Marketplace::cancel_order(RuntimeOrigin::signed(ALICE), order_id + 1),
			pallet_marketplace::Error::<Test, pallet_marketplace::Instance1>::InvalidOrderId
		);
		// can't cancel not own order
		assert_noop!(
			Marketplace::cancel_order(RuntimeOrigin::signed(BOB), order_id),
			pallet_marketplace::Error::<Test, pallet_marketplace::Instance1>::OrderCreatorMismatch
		);

		setup_secondary_market_tokens_and_balance(BOB);
		let bob_free_balance =
			<Test as pallet_insurances::Config>::StableCurrency::reducible_balance(
				USDT_ID,
				&BOB,
				Preservation::Preserve,
				Fortitude::Polite,
			);

		// test sell order partial fulfillment
		let first_fulfillment_amount = 10;
		let residual = order_token_balance - first_fulfillment_amount;
		assert_ok!(Marketplace::fulfill_order(
			RuntimeOrigin::signed(BOB),
			order_id,
			first_fulfillment_amount
		));
		System::assert_has_event(RuntimeEvent::Marketplace(pallet_marketplace::Event::<
			Test,
			pallet_marketplace::Instance1,
		>::OrderPartiallyFulfilled {
			id: order_id,
			who: BOB,
			amount: 10,
			residual: residual.clone(),
		}));
		assert_eq!(
			<Test as pallet_insurances::Config>::StableCurrency::reducible_balance(
				USDT_ID,
				&BOB,
				Preservation::Preserve,
				Fortitude::Polite,
			),
			bob_free_balance - 10 * order_info.price_per_token
		);

		// test sell order partial fulfillment
		let second_fulfillment_amount = 30;
		let residual = residual - second_fulfillment_amount;
		assert_ok!(Marketplace::fulfill_order(
			RuntimeOrigin::signed(BOB),
			order_id,
			second_fulfillment_amount
		));
		System::assert_has_event(RuntimeEvent::Marketplace(pallet_marketplace::Event::<
			Test,
			pallet_marketplace::Instance1,
		>::OrderPartiallyFulfilled {
			id: order_id,
			who: BOB,
			amount: 30,
			residual,
		}));
		assert_eq!(
			<Test as pallet_insurances::Config>::StableCurrency::reducible_balance(
				USDT_ID,
				&BOB,
				Preservation::Preserve,
				Fortitude::Polite,
			),
			bob_free_balance - 40 * order_info.price_per_token
		);

		assert_ok!(Marketplace::cancel_order(RuntimeOrigin::signed(ALICE), order_id));
		assert_eq!(
			<Test as pallet_insurances::Config>::StableCurrency::reducible_balance(
				USDT_ID,
				&BOB,
				Preservation::Preserve,
				Fortitude::Polite,
			),
			bob_free_balance
		);
		assert_eq!(
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(token_id, ALICE),
			token_balance
		);

		System::assert_has_event(RuntimeEvent::Marketplace(pallet_marketplace::Event::<
			Test,
			pallet_marketplace::Instance1,
		>::OrderCanceled {
			id: order_id,
		}));

		assert_eq!(
			pallet_marketplace::OrderBook::<Test, pallet_marketplace::Instance1>::iter().count(),
			0
		);
	});
}

#[test]
fn cancel_order_not_enough_reserved_funds() {
	new_test_ext().execute_with(|| {
		let token_id = setup_testing_environment();

		setup_secondary_market_tokens_and_balance(BOB);

		let balance = <Test as pallet_insurances::Config>::StableCurrency::reducible_balance(
			USDT_ID,
			&ALICE,
			Preservation::Preserve,
			Fortitude::Polite,
		);
		let token_balance =
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(token_id, ALICE);

		let order_id = create_order(ALICE, token_id, 10, 100, OrderType::Buy);

		assert_eq!(
			<Test as pallet_insurances::Config>::StableCurrency::reducible_balance(
				USDT_ID,
				&ALICE,
				Preservation::Preserve,
				Fortitude::Polite,
			),
			balance - 1000
		);

		// can't cancel non-existent order
		assert_noop!(
			Marketplace::cancel_order(RuntimeOrigin::signed(ALICE), order_id + 1),
			pallet_marketplace::Error::<Test, pallet_marketplace::Instance1>::InvalidOrderId
		);
		// can't cancel not own order
		assert_noop!(
			Marketplace::cancel_order(RuntimeOrigin::signed(BOB), order_id),
			pallet_marketplace::Error::<Test, pallet_marketplace::Instance1>::OrderCreatorMismatch
		);
		assert_eq!(
			<Test as pallet_insurances::Config>::StableCurrency::increase_balance(
				USDT_ID,
				&ALICE,
				5 * 100,
				Precision::Exact
			)
			.unwrap(),
			5 * 100
		);
		assert_eq!(
			<Test as pallet_insurances::Config>::StableCurrency::reducible_balance(
				USDT_ID,
				&ALICE,
				Preservation::Preserve,
				Fortitude::Polite,
			),
			balance - 500
		);
		assert_ok!(Marketplace::cancel_order(RuntimeOrigin::signed(ALICE), order_id));

		assert_eq!(
			<Test as pallet_insurances::Config>::StableCurrency::reducible_balance(
				USDT_ID,
				&ALICE,
				Preservation::Preserve,
				Fortitude::Polite,
			),
			balance + 500
		);
		assert_eq!(
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(token_id, ALICE),
			token_balance
		);

		assert_eq!(
			pallet_marketplace::OrderBook::<Test, pallet_marketplace::Instance1>::iter().count(),
			0
		);
	});
}

#[test]
fn fulfill_order_works() {
	new_test_ext().execute_with(|| {
		let alice_token = setup_testing_environment();
		let alice_token_balance =
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, ALICE);
		let alice_order = create_order(ALICE, alice_token, 10, 100, OrderType::Sell);
		let alice_order_info =
			pallet_marketplace::OrderBook::<Test, pallet_marketplace::Instance1>::get(alice_order)
				.unwrap();
		let alice_total_balance = total_balance!(ALICE);

		setup_secondary_market_tokens_and_balance(BOB);
		let bob_order = create_order(BOB, alice_token, 20, 100, OrderType::Buy);
		let bob_order_info =
			pallet_marketplace::OrderBook::<Test, pallet_marketplace::Instance1>::get(bob_order)
				.unwrap();
		let bob_total_balance = total_balance!(BOB);
		let bob_token_balance =
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, BOB);

		// test sell order fulfillment
		assert_ok!(Marketplace::fulfill_order(RuntimeOrigin::signed(BOB), alice_order, 10));
		System::assert_has_event(RuntimeEvent::Marketplace(pallet_marketplace::Event::<
			Test,
			pallet_marketplace::Instance1,
		>::OrderFulfilled {
			id: alice_order,
			who: BOB,
		}));
		assert_eq!(
			total_balance!(ALICE),
			alice_total_balance + alice_order_info.token_amount * alice_order_info.price_per_token
		);
		assert_eq!(
			total_balance!(BOB),
			bob_total_balance - alice_order_info.token_amount * alice_order_info.price_per_token
		);
		assert_eq!(
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, ALICE),
			alice_token_balance - alice_order_info.token_amount
		);
		assert_eq!(
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, BOB),
			bob_token_balance + alice_order_info.token_amount
		);

		let alice_total_balance = total_balance!(ALICE);
		let alice_token_balance =
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, ALICE);
		let bob_total_balance = total_balance!(BOB);
		let bob_token_balance =
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, BOB);

		// test buy order fulfillment
		assert_ok!(Marketplace::fulfill_order(RuntimeOrigin::signed(ALICE), bob_order, 20));
		System::assert_has_event(RuntimeEvent::Marketplace(pallet_marketplace::Event::<
			Test,
			pallet_marketplace::Instance1,
		>::OrderFulfilled {
			id: bob_order,
			who: ALICE,
		}));

		assert_eq!(
			total_balance!(ALICE),
			alice_total_balance + bob_order_info.token_amount * bob_order_info.price_per_token
		);
		assert_eq!(total_balance!(BOB), bob_total_balance);
		assert_eq!(
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, ALICE),
			alice_token_balance - bob_order_info.token_amount
		);
		assert_eq!(
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, BOB),
			bob_token_balance + bob_order_info.token_amount
		);
	});
}

#[test]
fn fulfill_order_invalid_tokens_amount() {
	new_test_ext().execute_with(|| {
		let alice_token = setup_testing_environment();
		let alice_order = create_order(ALICE, alice_token, 10, 100, OrderType::Sell);

		// can't fulfill with zero amount
		assert_noop!(
			Marketplace::fulfill_order(RuntimeOrigin::signed(BOB), alice_order + 1, 0),
			pallet_marketplace::Error::<Test, pallet_marketplace::Instance1>::InvalidTokensAmount
		);
	});
}

#[test]
fn fulfill_order_invalid_order_id() {
	new_test_ext().execute_with(|| {
		let alice_token = setup_testing_environment();
		let alice_token_balance =
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, ALICE);
		let alice_order = create_order(ALICE, alice_token, 10, 100, OrderType::Sell);

		// can't fulfill non-existent order
		assert_noop!(
			Marketplace::fulfill_order(
				RuntimeOrigin::signed(BOB),
				alice_order + 1,
				alice_token_balance
			),
			pallet_marketplace::Error::<Test, pallet_marketplace::Instance1>::InvalidOrderId
		);
	});
}

#[test]
fn fulfill_order_not_enough_funds() {
	new_test_ext().execute_with(|| {
		let alice_token = setup_secondary_market_tokens_and_balance(ALICE);
		let alice_order = create_order(ALICE, alice_token, 10, 100, OrderType::Sell);

		// can't fulfill without enough amount of funds
		assert_noop!(
			Marketplace::fulfill_order(RuntimeOrigin::signed(BOB), alice_order, 1),
			pallet_marketplace::Error::<Test, pallet_marketplace::Instance1>::NotEnoughFunds
		);
	});
}

#[test]
fn fulfill_order_buy_unreserve_failure() {
	new_test_ext().execute_with(|| {
		let alice_token = setup_testing_environment();
		let alice_balance = <Test as pallet_insurances::Config>::StableCurrency::reducible_balance(
			USDT_ID,
			&ALICE,
			Preservation::Preserve,
			Fortitude::Polite,
		);
		setup_secondary_market_tokens_and_balance(BOB);

		let alice_order_sell = create_order(ALICE, alice_token, 10, 100, OrderType::Sell);
		let alice_order_sell_info = pallet_marketplace::OrderBook::<
			Test,
			pallet_marketplace::Instance1,
		>::get(alice_order_sell)
		.unwrap();

		assert_ok!(Marketplace::fulfill_order(
			RuntimeOrigin::signed(BOB),
			alice_order_sell,
			alice_order_sell_info.token_amount
		));

		let alice_order_buy = create_order(ALICE, alice_token, 10, 100, OrderType::Buy);
		let alice_order_buy_info = pallet_marketplace::OrderBook::<
			Test,
			pallet_marketplace::Instance1,
		>::get(alice_order_buy)
		.unwrap();

		assert_eq!(
			<Test as pallet_insurances::Config>::StableCurrency::increase_balance(
				USDT_ID,
				&ALICE,
				5 * alice_order_buy_info.price_per_token,
				Precision::Exact
			)
			.unwrap(),
			5 * alice_order_buy_info.price_per_token
		);
		assert_eq!(
			<Test as pallet_insurances::Config>::StableCurrency::reducible_balance(
				USDT_ID,
				&ALICE,
				Preservation::Preserve,
				Fortitude::Polite,
			),
			alice_balance + 500
		);
		assert_ok!(Marketplace::fulfill_order(
			RuntimeOrigin::signed(BOB),
			alice_order_buy,
			alice_order_buy_info.token_amount
		));
		assert_eq!(
			<Test as pallet_insurances::Config>::StableCurrency::reducible_balance(
				USDT_ID,
				&ALICE,
				Preservation::Preserve,
				Fortitude::Polite,
			),
			alice_balance + 500
		);
	});
}

#[test]
fn partial_fulfill_order_works() {
	new_test_ext().execute_with(|| {
		let alice_token = setup_testing_environment();
		let alice_token_balance =
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, ALICE);
		let alice_order =
			create_order(ALICE, alice_token, alice_token_balance, 100, OrderType::Sell);
		let alice_order_info =
			pallet_marketplace::OrderBook::<Test, pallet_marketplace::Instance1>::get(alice_order)
				.unwrap();
		assert_eq!(
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, ALICE),
			alice_token_balance - alice_order_info.token_amount
		);
		let alice_total_balance = total_balance!(ALICE);

		setup_secondary_market_tokens_and_balance(BOB);
		let bob_total_balance = total_balance!(BOB);
		let bob_token_balance =
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, BOB);

		// test sell order partial fulfillment
		let first_fulfillment_amount = 10;
		let residual = alice_token_balance - first_fulfillment_amount;
		assert_ok!(Marketplace::fulfill_order(
			RuntimeOrigin::signed(BOB),
			alice_order,
			first_fulfillment_amount
		));
		System::assert_has_event(RuntimeEvent::Marketplace(pallet_marketplace::Event::<
			Test,
			pallet_marketplace::Instance1,
		>::OrderPartiallyFulfilled {
			id: alice_order,
			who: BOB,
			amount: 10,
			residual: residual.clone(),
		}));
		assert_eq!(total_balance!(BOB), bob_total_balance - 10 * alice_order_info.price_per_token);

		assert_eq!(total_balance!(ALICE), alice_total_balance);
		assert_eq!(
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, BOB),
			bob_token_balance
		);

		// test sell order partial fulfillment
		let second_fulfillment_amount = 40;
		let residual = residual - second_fulfillment_amount;
		assert_ok!(Marketplace::fulfill_order(
			RuntimeOrigin::signed(BOB),
			alice_order,
			second_fulfillment_amount
		));
		System::assert_has_event(RuntimeEvent::Marketplace(pallet_marketplace::Event::<
			Test,
			pallet_marketplace::Instance1,
		>::OrderPartiallyFulfilled {
			id: alice_order,
			who: BOB,
			amount: 40,
			residual,
		}));
		assert_eq!(total_balance!(BOB), bob_total_balance - 50 * alice_order_info.price_per_token);

		assert_eq!(total_balance!(ALICE), alice_total_balance);
		assert_eq!(
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, BOB),
			bob_token_balance
		);

		assert_ok!(Marketplace::fulfill_order(RuntimeOrigin::signed(BOB), alice_order, 50));
		System::assert_has_event(RuntimeEvent::Marketplace(pallet_marketplace::Event::<
			Test,
			pallet_marketplace::Instance1,
		>::OrderFulfilled {
			id: alice_order,
			who: BOB,
		}));

		assert_eq!(
			total_balance!(ALICE),
			alice_total_balance + alice_order_info.token_amount * alice_order_info.price_per_token
		);

		assert_eq!(
			total_balance!(BOB),
			bob_total_balance - alice_order_info.token_amount * alice_order_info.price_per_token
		);
		assert_eq!(
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, BOB),
			bob_token_balance + alice_order_info.token_amount
		);
	});
}

#[test]
fn exceeds_max_fulfillers_count() {
	new_test_ext().execute_with(|| {
		let alice_token = setup_testing_environment();
		let alice_token_balance =
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, ALICE);
		let alice_order =
			create_order(ALICE, alice_token, alice_token_balance, 100, OrderType::Sell);
		let alice_order_info =
			pallet_marketplace::OrderBook::<Test, pallet_marketplace::Instance1>::get(alice_order)
				.unwrap();
		assert_eq!(
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, ALICE),
			alice_token_balance - alice_order_info.token_amount
		);
		let alice_total_balance = total_balance!(ALICE);

		setup_secondary_market_tokens_and_balance(BOB);
		let bob_total_balance = total_balance!(BOB);
		let bob_token_balance =
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, BOB);

		// test sell order partial fulfillment
		let first_fulfillment_amount = 10;
		let residual = alice_token_balance - first_fulfillment_amount;
		assert_ok!(Marketplace::fulfill_order(
			RuntimeOrigin::signed(BOB),
			alice_order,
			first_fulfillment_amount
		));
		System::assert_has_event(RuntimeEvent::Marketplace(pallet_marketplace::Event::<
			Test,
			pallet_marketplace::Instance1,
		>::OrderPartiallyFulfilled {
			id: alice_order,
			who: BOB,
			amount: 10,
			residual: residual.clone(),
		}));
		assert_eq!(total_balance!(BOB), bob_total_balance - 10 * alice_order_info.price_per_token);

		assert_eq!(total_balance!(ALICE), alice_total_balance);
		assert_eq!(
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, BOB),
			bob_token_balance
		);

		// test sell order partial fulfillment
		let second_fulfillment_amount = 10;
		let residual = residual - second_fulfillment_amount;
		assert_ok!(Marketplace::fulfill_order(
			RuntimeOrigin::signed(BOB),
			alice_order,
			second_fulfillment_amount
		));
		System::assert_has_event(RuntimeEvent::Marketplace(pallet_marketplace::Event::<
			Test,
			pallet_marketplace::Instance1,
		>::OrderPartiallyFulfilled {
			id: alice_order,
			who: BOB,
			amount: 10,
			residual: residual.clone(),
		}));
		assert_eq!(total_balance!(BOB), bob_total_balance - 20 * alice_order_info.price_per_token);

		assert_eq!(total_balance!(ALICE), alice_total_balance);
		assert_eq!(
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, BOB),
			bob_token_balance
		);

		let third_fulfillment_amount = 10;
		let residual = residual - third_fulfillment_amount;
		assert_ok!(Marketplace::fulfill_order(
			RuntimeOrigin::signed(BOB),
			alice_order,
			third_fulfillment_amount
		));
		System::assert_has_event(RuntimeEvent::Marketplace(pallet_marketplace::Event::<
			Test,
			pallet_marketplace::Instance1,
		>::OrderPartiallyFulfilled {
			id: alice_order,
			who: BOB,
			amount: 10,
			residual,
		}));
		assert_eq!(total_balance!(BOB), bob_total_balance - 30 * alice_order_info.price_per_token);
		assert_eq!(total_balance!(ALICE), alice_total_balance);
		assert_eq!(
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, BOB),
			bob_token_balance
		);

		// test sell order partial fulfillment
		assert_noop!(
			Marketplace::fulfill_order(RuntimeOrigin::signed(BOB), alice_order, 10),
			Error::<Test, pallet_marketplace::Instance1>::ExceedsMaxFulfillersCount
		);
	});
}
