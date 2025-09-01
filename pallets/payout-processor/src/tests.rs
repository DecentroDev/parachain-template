use crate::{self as pallet_payout_processor, mock::*, Pallet};
use frame_support::{
	assert_err, assert_ok,
	traits::{
		fungibles::Inspect,
		tokens::{Fortitude, Preservation},
		Currency, Hooks,
	},
};
use sp_runtime::traits::Hash;

use pallet_dao::NextProposalIndex;
use pallet_insurances::types::{ContractLink, InsuranceMetadata, InsuranceStatus, InsuranceType};
use pallet_marketplace::{types::OrderType, OrderBook, OrderSMTsAmount};

type AccountId = <Test as frame_system::Config>::AccountId;
type AssetId = <Test as pallet_insurances::Config>::AssetId;
type Balance = <Test as pallet_insurances::Config>::Balance;
type BlockNumber = <Test as frame_system::Config>::BlockNumber;
type StringLimit = <Test as pallet_insurances::Config>::StringLimit;

const USDT_ID: u32 = 1984;
const INITIAL_BALANCE: u128 = 1_000_000;

fn create_order(
	creator: AccountId,
	token_id: AssetId,
	token_amount: Balance,
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

fn fulfill_metadata(
	name: Option<InsuranceType>,
	creator: Option<AccountId>,
	underwrite_amount: Option<u128>,
	premium_amount: Option<u128>,
	starts_on: Option<u64>,
	ends_on: Option<u64>,
) -> InsuranceMetadata<Balance, AccountId, BlockNumber, AssetId, ContractLink<u8, StringLimit>> {
	InsuranceMetadata {
		name: name.unwrap_or(InsuranceType::Cyclone),
		location: 1,
		creator: creator.unwrap_or(BOB),
		status: InsuranceStatus::Active,
		underwrite_amount: underwrite_amount.unwrap_or(10_000),
		premium_amount: premium_amount.unwrap_or(100),
		contract_link: Default::default(),
		starts_on: starts_on.unwrap_or(10),
		ends_on: ends_on.unwrap_or(20),
		smt_id: None,
	}
}

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
		assert_ok!(Assets::touch(RuntimeOrigin::signed(88), USDT_ID));
		assert_ok!(<Test as pallet_insurances::Config>::StableCurrency::mint(
			RuntimeOrigin::signed(88),
			USDT_ID,
			88,
			INITIAL_BALANCE
		));
	}

	Balances::make_free_balance_be(&beneficiary, INITIAL_BALANCE);
	assert_ok!(<Test as pallet_insurances::Config>::StableCurrency::mint(
		RuntimeOrigin::signed(88),
		USDT_ID,
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
fn handle_event_works() {
	new_test_ext().execute_with(|| {
		setup_testing_environment();

		let root_operator =
			<Test as orml_oracle::Config<orml_oracle::Instance1>>::RootOperatorAccountId::get();
		let dao_account_id = Dao::pallet_account_id().unwrap(); // is always Some, provided in genesis config

		assert_ok!(Dao::add_member(RuntimeOrigin::root(), ALICE));

		let initial_bob_balance = total_balance!(BOB);
		let initial_dao_balance = total_balance!(dao_account_id);

		let collection_id = 0;
		let insurance_id = 0;
		let premium_amount = 100;
		let underwrite_amount = 10_000;
		let starts_on = 10;

		let metadata = fulfill_metadata(None, Some(BOB), None, None, None, None);

		let proposal =
			RuntimeCall::Dao(pallet_dao::Call::allocate_liquidity { metadata: metadata.clone() });

		let proposal_hash = <Test as frame_system::Config>::Hashing::hash_of(&proposal);

		assert_ok!(Dao::request_insurance(
			RuntimeOrigin::signed(BOB),
			metadata.clone(),
			Box::new(proposal)
		));
		let proposal_index = System::events()
			.iter()
			.rev()
			.find_map(|event| {
				if let RuntimeEvent::Collective(ref event) = event.event {
					if let pallet_collective::Event::Proposed { proposal_index, .. } = event {
						Some(proposal_index.clone())
					} else {
						None
					}
				} else {
					None
				}
			})
			.unwrap();
		assert_ok!(Dao::vote(RuntimeOrigin::signed(ALICE), proposal_hash, proposal_index, true));
		frame_system::Pallet::<Test>::set_block_number(5);
		<pallet_dao::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);

		frame_system::Pallet::<Test>::set_block_number(starts_on + 1);
		<crate::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);
		System::assert_has_event(RuntimeEvent::PayoutProcessor(crate::Event::<
			Test,
			crate::Instance1,
		>::InsuranceActivated {
			collection_id: collection_id.clone(),
			insurance_id: insurance_id.clone(),
			metadata: metadata.clone(),
		}));

		assert_ok!(crate::Pallet::<Test, crate::Instance1>::feed_event(
			RuntimeOrigin::root(),
			(InsuranceType::Cyclone, 1_u8),
			vec![],
		),);

		System::assert_has_event(RuntimeEvent::PayoutProcessor(crate::Event::<
			Test,
			crate::Instance1,
		>::HandledInsuranceEvent {
			who: root_operator,
			event: InsuranceType::Cyclone,
			location: 1_u8,
			insurance: None,
		}));
		System::assert_has_event(RuntimeEvent::PayoutProcessor(
			crate::Event::<Test, _>::PaidOutInsurance {
				beneficiary: BOB,
				event: InsuranceType::Cyclone,
				collection_id,
				insurance_id,
				metadata: InsuranceMetadata { status: InsuranceStatus::PaidOut, ..metadata },
			},
		));
		System::assert_has_event(RuntimeEvent::Insurances(
			pallet_insurances::Event::<Test>::InsuranceDestroyed { collection_id, insurance_id },
		));

		assert_eq!(
			total_balance!(BOB),
			initial_bob_balance + underwrite_amount - premium_amount,
			"Invalid user balance"
		);
		assert_eq!(
			total_balance!(dao_account_id),
			initial_dao_balance - underwrite_amount,
			"Invalid treasury balance"
		);

		use frame_support::traits::tokens::nonfungibles::InspectEnumerable;
		assert_eq!(<Test as pallet_insurances::Config>::InsuredToken::collections().count(), 0);
	});
}

#[test]
fn insurance_destroyed_on_expiration() {
	new_test_ext().execute_with(|| {
		setup_testing_environment();

		let dao_account_id = Dao::pallet_account_id().unwrap(); // is always Some, provided in genesis config
		let dao_initial_balance = total_balance!(dao_account_id);
		let bob_initial_balance = total_balance!(BOB);

		assert_ok!(Dao::add_member(RuntimeOrigin::root(), ALICE));

		let starts_on = 5;
		let ends_on = 20;
		let premium_amount = 100;
		let underwrite_amount = 10_000;

		let metadata = InsuranceMetadata {
			name: InsuranceType::Cyclone,
			location: 1,
			creator: BOB,
			status: InsuranceStatus::Active,
			underwrite_amount,
			premium_amount,
			contract_link: Default::default(),
			starts_on,
			ends_on,
			smt_id: None,
		};

		let proposal =
			RuntimeCall::Dao(pallet_dao::Call::allocate_liquidity { metadata: metadata.clone() });

		let proposal_hash = <Test as frame_system::Config>::Hashing::hash_of(&proposal);

		assert_ok!(Dao::request_insurance(
			RuntimeOrigin::signed(BOB),
			metadata.clone(),
			Box::new(proposal)
		));
		let proposal_index = System::events()
			.iter()
			.rev()
			.find_map(|event| {
				if let RuntimeEvent::Collective(ref event) = event.event {
					if let pallet_collective::Event::Proposed { proposal_index, .. } = event {
						Some(proposal_index.clone())
					} else {
						None
					}
				} else {
					None
				}
			})
			.unwrap();
		assert_ok!(Dao::vote(RuntimeOrigin::signed(ALICE), proposal_hash, proposal_index, true));

		frame_system::Pallet::<Test>::set_block_number(5);
		<pallet_dao::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);

		frame_system::Pallet::<Test>::set_block_number(ends_on + 5);
		<crate::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);

		System::assert_has_event(RuntimeEvent::Insurances(
			pallet_insurances::Event::<Test>::InsuranceDestroyed {
				collection_id: 0,
				insurance_id: 0,
			},
		));

		// insurance expired, the storage must be empty
		assert_eq!(pallet_insurances::Metadata::<Test>::iter().count(), 0);

		// user lost the premium amount and collection deposit
		// no additional funds are reserved
		let collection_deposit = <Test as pallet_uniques::Config>::CollectionDeposit::get();

		assert_eq!(total_balance!(BOB), bob_initial_balance - premium_amount - collection_deposit);

		assert_ok!(<Test as pallet_insurances::Config>::StableCurrency::transfer(
			RuntimeOrigin::signed(BOB),
			USDT_ID,
			dao_account_id,
			premium_amount
		));

		// dao profited from expired insurance
		assert_eq!(total_balance!(dao_account_id), dao_initial_balance + premium_amount);
	})
}

#[test]
fn dao_paid_out_on_insurance_expiration() {
	new_test_ext().execute_with(|| {
		setup_testing_environment();

		let dao_account_id = pallet_dao::PalletAccountId::<Test, _>::get().unwrap();
		let dao_initial_balance = total_balance!(dao_account_id);

		assert_ok!(Dao::add_member(RuntimeOrigin::root(), ALICE));

		// (user_id, underwrite_amount)
		let user_definitions =
			[(10, 10000), (11, 20000), (12, 30000), (13, 40000), (14, 50000), (15, 60000)];
		let ends_on = 20;
		let premium_amount = 10_000;
		let user_initial_balance = 2 * premium_amount;

		// setup insurance requests and approve all
		for (user_id, underwrite_amount) in user_definitions {
			Balances::make_free_balance_be(&user_id, INITIAL_BALANCE);
			assert_ok!(<Test as pallet_insurances::Config>::StableCurrency::mint(
				RuntimeOrigin::signed(88),
				USDT_ID,
				user_id,
				user_initial_balance
			));
			let metadata = fulfill_metadata(
				None,
				Some(user_id),
				Some(underwrite_amount),
				Some(premium_amount),
				None,
				Some(ends_on),
			);

			let proposal = RuntimeCall::Dao(pallet_dao::Call::allocate_liquidity {
				metadata: metadata.clone(),
			});

			let proposal_hash = <Test as frame_system::Config>::Hashing::hash_of(&proposal);

			assert_ok!(Dao::request_insurance(
				RuntimeOrigin::signed(user_id),
				metadata.clone(),
				Box::new(proposal)
			));
			let proposal_index = System::events()
				.iter()
				.rev()
				.find_map(|event| {
					if let RuntimeEvent::Collective(ref event) = event.event {
						if let pallet_collective::Event::Proposed { proposal_index, .. } = event {
							Some(proposal_index.clone())
						} else {
							None
						}
					} else {
						None
					}
				})
				.unwrap();
			assert_ok!(Dao::vote(
				RuntimeOrigin::signed(ALICE),
				proposal_hash,
				proposal_index,
				true
			));
		}

		frame_system::Pallet::<Test>::set_block_number(5);
		<pallet_dao::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);

		frame_system::Pallet::<Test>::set_block_number(ends_on + 5);
		<crate::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);
		// all insurances expired, the storage must be empty
		assert_eq!(pallet_insurances::Metadata::<Test>::iter().count(), 0);

		// all users lost the premium amount and collection deposit
		// no additional funds are reserved
		let collection_deposit = <Test as pallet_uniques::Config>::CollectionDeposit::get();

		// dao profited from all expired insurances
		for (user_id, _) in user_definitions {
			assert_eq!(
				total_balance!(user_id),
				user_initial_balance - premium_amount - collection_deposit - 1
			);

			assert_ok!(<Test as pallet_insurances::Config>::StableCurrency::transfer(
				RuntimeOrigin::signed(BOB),
				USDT_ID,
				dao_account_id,
				premium_amount
			));
		}
		assert_eq!(
			total_balance!(dao_account_id),
			dao_initial_balance + user_definitions.len() as u128 * premium_amount
		);
	})
}

#[test]
fn user_gets_paid_out() {
	new_test_ext().execute_with(|| {
		setup_testing_environment();

		let dao_account_id = pallet_dao::PalletAccountId::<Test, _>::get().unwrap();

		let dao_initial_balance = total_balance!(dao_account_id);

		assert_ok!(Dao::add_member(RuntimeOrigin::root(), ALICE));

		// (user_id, underwrite_amount)
		let user_definitions =
			[(10, 10_000), (11, 20_000), (12, 30_000), (13, 40_000), (14, 50_000), (15, 60_000)];
		let starts_on = 5;
		let ends_on = 20;
		let premium_amount = 10_000;
		let user_initial_balance = 2 * premium_amount;

		// setup insurance requests and approve all
		// even ones are InsuranceType::Cyclone, odd ones are InsuranceType::Earthquake
		for (i, (user_id, underwrite_amount)) in user_definitions.iter().enumerate() {
			Balances::make_free_balance_be(&user_id, INITIAL_BALANCE);
			assert_ok!(<Test as pallet_insurances::Config>::StableCurrency::mint(
				RuntimeOrigin::signed(88),
				USDT_ID,
				*user_id,
				user_initial_balance
			));

			assert_eq!(total_balance!(user_id), user_initial_balance - 1);

			let metadata = fulfill_metadata(
				Some(if i % 2 == 0 { InsuranceType::Cyclone } else { InsuranceType::Earthquake }),
				Some(user_id.clone()),
				Some(underwrite_amount.clone()),
				Some(premium_amount),
				Some(starts_on),
				Some(ends_on),
			);

			let proposal = RuntimeCall::Dao(pallet_dao::Call::allocate_liquidity {
				metadata: metadata.clone(),
			});
			let proposal_hash = <Test as frame_system::Config>::Hashing::hash_of(&proposal);

			assert_ok!(Dao::request_insurance(
				RuntimeOrigin::signed(*user_id),
				metadata.clone(),
				Box::new(proposal)
			));
			let proposal_index = System::events()
				.iter()
				.rev()
				.find_map(|event| {
					if let RuntimeEvent::Collective(ref event) = event.event {
						if let pallet_collective::Event::Proposed { proposal_index, .. } = event {
							Some(proposal_index.clone())
						} else {
							None
						}
					} else {
						None
					}
				})
				.unwrap();
			assert_ok!(Dao::vote(
				RuntimeOrigin::signed(ALICE),
				proposal_hash,
				proposal_index,
				true
			));
		}

		frame_system::Pallet::<Test>::set_block_number(5);
		<pallet_dao::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);

		frame_system::Pallet::<Test>::set_block_number(starts_on + 3);
		// feeding Tsunami event, ceil(user_definitions.len() / 2) users will be affected
		assert_ok!(crate::Pallet::<Test, _>::feed_event(
			RuntimeOrigin::root(),
			(InsuranceType::Cyclone, 1_u8),
			vec![],
		));
		// metadata storage contains only unaffected insurances
		assert_eq!(pallet_insurances::Metadata::<Test>::iter().count(), user_definitions.len() / 2);

		// affected users are paid out with their respective underwrite_amount
		// no additional user funds are reserved
		for (user_id, underwrite_amount) in user_definitions.iter().step_by(2) {
			assert_eq!(
				total_balance!(user_id),
				user_initial_balance + underwrite_amount - premium_amount - 1
			);
		}

		let dao_balance_after_payout =
			dao_initial_balance - user_definitions.iter().step_by(2).map(|x| x.1).sum::<u128>();
		// assert_eq!(total_balance!(dao_account_id), dao_balance_after_payout);

		frame_system::Pallet::<Test>::set_block_number(ends_on + 5);
		<crate::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);

		// unaffected users lose premium amount
		// no additional user funds are reserved
		for (user_id, _) in user_definitions.iter().skip(1).step_by(2) {
			assert_eq!(total_balance!(user_id), user_initial_balance - premium_amount - 1);
			assert_ok!(<Test as pallet_insurances::Config>::StableCurrency::transfer(
				RuntimeOrigin::signed(BOB),
				USDT_ID,
				dao_account_id,
				premium_amount
			));
		}

		let dao_balance_after_premium_collect = dao_balance_after_payout +
			user_definitions.iter().skip(1).step_by(2).count() as u128 * premium_amount;
		assert_eq!(total_balance!(dao_account_id), dao_balance_after_premium_collect);
	})
}

#[test]
fn liquidity_providers_get_paid_out() {
	new_test_ext().execute_with(|| {
		setup_testing_environment();

		assert_ok!(Dao::add_member(RuntimeOrigin::root(), ALICE));

		let user_id = ALICE;
		let dao_account_id = Dao::pallet_account_id().unwrap();

		let dao_initial_balance = total_balance!(dao_account_id);

		// needed to guarantee premium amount is divisible by 100
		const PREMIUM_MULTIPLIER: u128 = 100;
		// the ratio between underwrite amount and premium amount
		// underwrite_amount = premium_amount * UNDERWRITE_COEF
		let underwrite_coef: u128 = 10;

		// construct a list of liquidity providers of an arbitrary length
		// in the end the amount of tokens each lp will hold is as follows
		// [1 * UNDERWRITE_COEF, 2 * UNDERWRITE_COEF, ..., (lp_count - 1) * UNDERWRITE_COEF]
		let lp_count = 42;
		let liquidity_providers: Vec<_> = (10..).take(lp_count).collect();
		println!("{:?}", liquidity_providers);

		let premium_amount = PREMIUM_MULTIPLIER * (1 + lp_count as u128) * lp_count as u128 / 2;
		let underwrite_amount = premium_amount * underwrite_coef;

		let lp_initial_balance = 1_000_000;

		let ends_on = 20;

		let metadata = fulfill_metadata(
			None,
			Some(user_id),
			Some(underwrite_amount),
			Some(premium_amount),
			None,
			Some(ends_on),
		);

		let proposal =
			RuntimeCall::Dao(pallet_dao::Call::allocate_liquidity { metadata: metadata.clone() });

		let proposal_hash = <Test as frame_system::Config>::Hashing::hash_of(&proposal);

		assert_ok!(Dao::request_insurance(
			RuntimeOrigin::signed(user_id),
			metadata.clone(),
			Box::new(proposal)
		));
		let proposal_index = System::events()
			.iter()
			.rev()
			.find_map(|event| {
				if let RuntimeEvent::Collective(ref event) = event.event {
					if let pallet_collective::Event::Proposed { proposal_index, .. } = event {
						Some(proposal_index.clone())
					} else {
						None
					}
				} else {
					None
				}
			})
			.unwrap();
		assert_ok!(Dao::vote(RuntimeOrigin::signed(ALICE), proposal_hash, proposal_index, true));
		frame_system::Pallet::<Test>::set_block_number(10);
		<pallet_dao::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);
		let (collection_id, insurance_id) = System::events()
			.iter()
			.rev()
			.find_map(|event| {
				if let RuntimeEvent::Dao(pallet_dao::Event::LiquidityAllocated {
					collection_id,
					item_id,
					..
				}) = event.event
				{
					Some((collection_id, item_id))
				} else {
					None
				}
			})
			.unwrap();

		// Provide liquidity to the first LP in the list
		Balances::make_free_balance_be(&liquidity_providers[0], 10);
		assert_ok!(<Test as pallet_insurances::Config>::StableCurrency::mint(
			RuntimeOrigin::signed(88),
			USDT_ID,
			liquidity_providers[0],
			lp_initial_balance,
		));
		assert_ok!(
			pallet_marketplace::Pallet::<Test, pallet_marketplace::Instance1>::provide_liquidity(
				RuntimeOrigin::signed(liquidity_providers[0]),
				collection_id,
				insurance_id,
			)
		);
		let first_provider = liquidity_providers[0];
		assert_eq!(total_balance!(first_provider), lp_initial_balance - underwrite_amount - 1);

		// After providing liquidity free balance of DAO must be the same as initial
		assert_eq!(total_balance!(dao_account_id), dao_initial_balance);

		for lp in liquidity_providers.iter().skip(1) {
			Balances::make_free_balance_be(&lp, 10);
			assert_ok!(<Test as pallet_insurances::Config>::StableCurrency::mint(
				RuntimeOrigin::signed(88),
				USDT_ID,
				lp.clone(),
				lp_initial_balance
			));
		}

		let smt_id = System::events()
			.iter()
			.rev()
			.find_map(|event| {
				if let RuntimeEvent::Marketplace(pallet_marketplace::Event::LiquidityProvided {
					smt_id,
					..
				}) = event.event
				{
					Some(smt_id)
				} else {
					None
				}
			})
			.unwrap();

		// redistribute the SMTs between all LPs in a way described previously
		for (i, lp) in liquidity_providers.iter().enumerate().skip(1) {
			assert_ok!(pallet_assets::Pallet::<Test>::transfer(
				RuntimeOrigin::signed(liquidity_providers[0]),
				smt_id.into(),
				*lp,
				((i + 1) as u128 * underwrite_coef).into(),
			));
		}

		for (i, lp) in liquidity_providers.iter().enumerate() {
			assert_eq!(Assets::balance(smt_id, lp), (i + 1) as u128 * underwrite_coef);
		}

		frame_system::Pallet::<Test>::set_block_number(ends_on);
		<Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);

		assert_eq!(
			pallet_insurances::Metadata::<Test>::get(collection_id, insurance_id)
				.unwrap()
				.status,
			pallet_insurances::types::InsuranceStatus::PayoutPending
		);

		// DAO account's has all USDT's because payouts just increase balance to holders
		assert_eq!(total_balance!(dao_account_id), dao_initial_balance);

		let initial_mint = underwrite_amount / 100;
		for (i, lp) in liquidity_providers.iter().enumerate().skip(1) {
			assert_ok!(Pallet::<Test, _>::claim_premium_payout(
				RuntimeOrigin::signed(*lp),
				smt_id.into()
			));
			println!("Balance of {lp:?} = {:?}", total_balance!(lp));
			assert_eq!(
				total_balance!(lp),
				lp_initial_balance - 1 +
					(i + 1) as u128 * (premium_amount + underwrite_amount) * underwrite_coef /
						initial_mint
			);
		}

		assert_eq!(total_balance!(dao_account_id), dao_initial_balance);
	});
}

#[test]
fn liquidity_providers_get_premium_paid_out() {
	new_test_ext().execute_with(|| {
		setup_testing_environment();

		assert_ok!(Dao::add_member(RuntimeOrigin::root(), ALICE));

		let dao_account_id = Dao::pallet_account_id().unwrap();
		let user_id = ALICE;

		let dao_initial_balance = total_balance!(dao_account_id);

		// needed to guarantee premium amount is divisible by 100
		const PREMIUM_MULTIPLIER: u128 = 100;
		// the ratio between underwrite amount and premium amount
		// underwrite_amount = premium_amount * UNDERWRITE_COEF
		let underwrite_coef: u128 = 10;

		// construct a list of liquidity providers of an arbitrary length
		// in the end the amount of tokens each lp will hold is as follows
		// [1 * UNDERWRITE_COEF, 2 * UNDERWRITE_COEF, ..., (lp_count - 1) * UNDERWRITE_COEF]
		let lp_count = 42;
		let liquidity_providers: Vec<_> = (10..).take(lp_count).collect();

		let premium_amount = PREMIUM_MULTIPLIER * (1 + lp_count as u128) * lp_count as u128 / 2;
		let underwrite_amount = premium_amount * underwrite_coef;
		let starts_on = 10;
		let ends_on = 20;

		let metadata = fulfill_metadata(
			None,
			Some(user_id),
			Some(underwrite_amount),
			Some(premium_amount),
			None,
			Some(ends_on),
		);

		let proposal =
			RuntimeCall::Dao(pallet_dao::Call::allocate_liquidity { metadata: metadata.clone() });

		let proposal_hash = <Test as frame_system::Config>::Hashing::hash_of(&proposal);

		assert_ok!(Dao::request_insurance(
			RuntimeOrigin::signed(user_id),
			metadata.clone(),
			Box::new(proposal)
		));
		let proposal_index = System::events()
			.iter()
			.rev()
			.find_map(|event| {
				if let RuntimeEvent::Collective(ref event) = event.event {
					if let pallet_collective::Event::Proposed { proposal_index, .. } = event {
						Some(proposal_index.clone())
					} else {
						None
					}
				} else {
					None
				}
			})
			.unwrap();
		assert_ok!(Dao::vote(RuntimeOrigin::signed(ALICE), proposal_hash, proposal_index, true));
		frame_system::Pallet::<Test>::set_block_number(5);
		<pallet_dao::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);
		let (collection_id, insurance_id) = System::events()
			.iter()
			.rev()
			.find_map(|event| {
				if let RuntimeEvent::Dao(pallet_dao::Event::LiquidityAllocated {
					collection_id,
					item_id,
					..
				}) = event.event
				{
					Some((collection_id, item_id))
				} else {
					None
				}
			})
			.unwrap();

		// Provide liquidity to the first LP in the list
		Balances::make_free_balance_be(&liquidity_providers[0], 10);
		assert_ok!(<Test as pallet_insurances::Config>::StableCurrency::mint(
			RuntimeOrigin::signed(88),
			USDT_ID,
			liquidity_providers[0],
			underwrite_amount * 2
		));
		assert_ok!(
			pallet_marketplace::Pallet::<Test, pallet_marketplace::Instance1>::provide_liquidity(
				RuntimeOrigin::signed(liquidity_providers[0]),
				collection_id,
				insurance_id,
			)
		);
		// After providing liquidity free balance of DAO must be the same as initial
		assert_eq!(total_balance!(dao_account_id), dao_initial_balance);

		let lp_initial_balance = 1_000_000;
		for lp in liquidity_providers.iter() {
			Balances::make_free_balance_be(&lp, 10);
			assert_ok!(<Test as pallet_insurances::Config>::StableCurrency::mint(
				RuntimeOrigin::signed(88),
				USDT_ID,
				*lp,
				lp_initial_balance
			));
		}

		let smt_id = System::events()
			.iter()
			.rev()
			.find_map(|event| {
				if let RuntimeEvent::Marketplace(pallet_marketplace::Event::LiquidityProvided {
					smt_id,
					..
				}) = event.event
				{
					Some(smt_id)
				} else {
					None
				}
			})
			.unwrap();

		// redistribute the SMTs between all LPs in a way described previously
		for (i, lp) in liquidity_providers.iter().enumerate().skip(1) {
			assert_ok!(pallet_assets::Pallet::<Test>::transfer(
				RuntimeOrigin::signed(liquidity_providers[0]),
				smt_id.into(),
				*lp,
				((i + 1) as u128 * underwrite_coef).into(),
			));
		}

		for (i, lp) in liquidity_providers.iter().enumerate() {
			assert_eq!(Assets::balance(smt_id, lp), (i + 1) as u128 * underwrite_coef);
		}

		frame_system::Pallet::<Test>::set_block_number(starts_on);
		<pallet_dao::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);

		assert_ok!(crate::Pallet::<Test, crate::Instance1>::feed_event(
			RuntimeOrigin::root(),
			(InsuranceType::Cyclone, 1_u8),
			vec![],
		));

		assert_eq!(
			pallet_insurances::Metadata::<Test>::get(collection_id, insurance_id)
				.unwrap()
				.status,
			pallet_insurances::types::InsuranceStatus::PremiumPayoutPending
		);

		// DAO account's reserved balances is used to store pending secondary market payouts
		assert_eq!(total_balance!(dao_account_id), dao_initial_balance);

		let initial_mint = underwrite_amount / 100;
		for (i, lp) in liquidity_providers.iter().enumerate().skip(1) {
			assert_ok!(Pallet::<Test, _>::claim_premium_payout(
				RuntimeOrigin::signed(*lp),
				smt_id.into()
			));
			assert_eq!(
				total_balance!(lp),
				lp_initial_balance +
					(i + 1) as u128 * (premium_amount) * underwrite_coef / initial_mint -
					1
			);
		}

		assert_eq!(total_balance!(dao_account_id), dao_initial_balance);
	});
}

// This test covers a part of DAO functionality, but since we use payout
// processor's hooks, we decided to put it here
#[test]
fn do_claim_dao_profits_works() {
	new_test_ext().execute_with(|| {
		setup_testing_environment();

		assert_ok!(Dao::add_member(RuntimeOrigin::root(), ALICE));
		assert_eq!(total_balance!(ALICE), INITIAL_BALANCE - 1);
		assert_eq!(NextProposalIndex::<Test, _>::get(), 0);

		let dao_account_id = Dao::pallet_account_id().unwrap();
		let user_id = BOB;
		let underwrite_amount = 1_000;

		let metadata = fulfill_metadata(None, None, Some(underwrite_amount), None, None, None);

		let proposal =
			RuntimeCall::Dao(pallet_dao::Call::allocate_liquidity { metadata: metadata.clone() });

		let proposal_hash = <Test as frame_system::Config>::Hashing::hash_of(&proposal);

		assert_ok!(Dao::request_insurance(
			RuntimeOrigin::signed(user_id),
			metadata.clone(),
			Box::new(proposal)
		));
		System::assert_last_event(RuntimeEvent::Dao(pallet_dao::Event::InsuranceRequested {
			metadata: metadata.clone(),
			proposal_hash,
			proposal_index: 0,
		}));
		assert_ok!(Dao::vote(RuntimeOrigin::signed(ALICE), proposal_hash, 0, true));

		frame_system::Pallet::<Test>::set_block_number(5);
		<pallet_dao::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);
		System::assert_has_event(RuntimeEvent::Collective(pallet_collective::Event::Executed {
			result: Ok(()),
			proposal_hash,
		}));
		assert_eq!(total_balance!(dao_account_id), INITIAL_BALANCE - underwrite_amount - 1);

		frame_system::Pallet::<Test>::set_block_number(20);
		<crate::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);

		System::assert_has_event(RuntimeEvent::Dao(pallet_dao::Event::DaoClaimedProfits {
			collection_id: 0,
			item_id: 0,
		}));
		assert_eq!(total_balance!(ALICE), INITIAL_BALANCE + metadata.premium_amount - 1);
	});
}

#[test]
fn clean_order_after_expiring_works() {
	new_test_ext().execute_with(|| {
		setup_testing_environment();

		let metadata = fulfill_metadata(None, None, None, None, None, None);

		let proposal =
			RuntimeCall::Dao(pallet_dao::Call::allocate_liquidity { metadata: metadata.clone() });
		assert_ok!(Dao::request_insurance(
			RuntimeOrigin::signed(BOB),
			metadata,
			Box::new(proposal.clone())
		));

		let proposal_hash = <Test as frame_system::Config>::Hashing::hash_of(&proposal);

		frame_system::Pallet::<Test>::set_block_number(1);
		assert_ok!(Dao::add_member(RuntimeOrigin::root(), ALICE));
		assert_ok!(Dao::vote(RuntimeOrigin::signed(ALICE), proposal_hash, 0, true));

		System::assert_has_event(RuntimeEvent::Dao(pallet_dao::Event::LiquidityProvisionVoted {
			who: ALICE,
			proposal_index: 0,
			decision: true,
		}));

		frame_system::Pallet::<Test>::set_block_number(5);
		<pallet_dao::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);
		System::assert_last_event(RuntimeEvent::Collective(pallet_collective::Event::Executed {
			result: Ok(()),
			proposal_hash,
		}));
		frame_system::Pallet::<Test>::set_block_number(10);
		<pallet_dao::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);

		assert_ok!(Marketplace::provide_liquidity(RuntimeOrigin::signed(ALICE), 0, 0));
		let alice_token = 0;
		System::assert_has_event(RuntimeEvent::Marketplace(
			pallet_marketplace::Event::LiquidityProvided {
				who: ALICE,
				smt_id: alice_token,
				collection_id: 0,
				item_id: 0,
			},
		));

		let alice_token_balance =
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, ALICE);
		let alice_order_1 =
			create_order(ALICE, alice_token, alice_token_balance / 10, 100, OrderType::Sell);

		for _ in 0..4 {
			let _ =
				create_order(ALICE, alice_token, alice_token_balance / 10, 100, OrderType::Sell);
		}

		assert_eq!(OrderBook::<Test, pallet_marketplace::Instance1>::iter().count(), 5);

		let alice_order_info =
			OrderBook::<Test, pallet_marketplace::Instance1>::get(alice_order_1).unwrap();

		assert_eq!(
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, ALICE),
			alice_token_balance - alice_order_info.token_amount * 5
		);

		let alice_total_balance = total_balance!(ALICE);
		let bob_total_balance = total_balance!(BOB);

		let bob_token_balance =
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, BOB);

		// test sell order partial fulfillment
		let fulfillment_amount = 5;
		let residual = alice_token_balance / 10 - fulfillment_amount;
		assert_ok!(Marketplace::fulfill_order(
			RuntimeOrigin::signed(BOB),
			alice_order_1,
			fulfillment_amount
		));
		System::assert_has_event(RuntimeEvent::Marketplace(pallet_marketplace::Event::<
			Test,
			pallet_marketplace::Instance1,
		>::OrderPartiallyFulfilled {
			id: alice_order_1,
			who: BOB,
			amount: 5,
			residual,
		}));
		assert_eq!(total_balance!(BOB), bob_total_balance - 5 * alice_order_info.price_per_token);
		assert_eq!(total_balance!(ALICE), alice_total_balance);
		assert_eq!(
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, BOB),
			bob_token_balance
		);

		assert_eq!(OrderSMTsAmount::<Test, pallet_marketplace::Instance1>::iter().count(), 1);

		frame_system::Pallet::<Test>::set_block_number(21);
		<pallet_dao::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);
		<Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);

		for i in 0..5 {
			System::assert_has_event(RuntimeEvent::Marketplace(
				pallet_marketplace::Event::OrderCanceled { id: i },
			));
		}

		assert_eq!(OrderBook::<Test, pallet_marketplace::Instance1>::iter().count(), 0);
		assert_eq!(OrderSMTsAmount::<Test, pallet_marketplace::Instance1>::iter().count(), 0);
	});
}

#[test]
fn clean_order_when_event_happenned_works() {
	new_test_ext().execute_with(|| {
		setup_testing_environment();

		let metadata = fulfill_metadata(None, None, None, None, None, None);

		let proposal =
			RuntimeCall::Dao(pallet_dao::Call::allocate_liquidity { metadata: metadata.clone() });
		assert_ok!(Dao::request_insurance(
			RuntimeOrigin::signed(BOB),
			metadata,
			Box::new(proposal.clone())
		));

		let proposal_hash = <Test as frame_system::Config>::Hashing::hash_of(&proposal);

		frame_system::Pallet::<Test>::set_block_number(1);
		assert_ok!(Dao::add_member(RuntimeOrigin::root(), ALICE));
		assert_ok!(Dao::vote(RuntimeOrigin::signed(ALICE), proposal_hash, 0, true));

		System::assert_has_event(RuntimeEvent::Dao(pallet_dao::Event::LiquidityProvisionVoted {
			who: ALICE,
			proposal_index: 0,
			decision: true,
		}));

		frame_system::Pallet::<Test>::set_block_number(5);
		<pallet_dao::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);
		System::assert_last_event(RuntimeEvent::Collective(pallet_collective::Event::Executed {
			result: Ok(()),
			proposal_hash,
		}));
		frame_system::Pallet::<Test>::set_block_number(10);
		<pallet_dao::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);

		assert_ok!(Marketplace::provide_liquidity(RuntimeOrigin::signed(ALICE), 0, 0));
		let alice_token = 0;
		System::assert_has_event(RuntimeEvent::Marketplace(
			pallet_marketplace::Event::LiquidityProvided {
				who: ALICE,
				smt_id: alice_token,
				collection_id: 0,
				item_id: 0,
			},
		));

		let alice_token_balance =
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, ALICE);
		let alice_order =
			create_order(ALICE, alice_token, alice_token_balance / 10, 100, OrderType::Sell);
		for _ in 0..4 {
			let _ =
				create_order(ALICE, alice_token, alice_token_balance / 10, 100, OrderType::Sell);
		}

		assert_eq!(OrderBook::<Test, pallet_marketplace::Instance1>::iter().count(), 5);

		let alice_order_info =
			OrderBook::<Test, pallet_marketplace::Instance1>::get(alice_order).unwrap();

		assert_eq!(
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, ALICE),
			alice_token_balance - alice_order_info.token_amount * 5
		);

		let alice_total_balance = total_balance!(ALICE);

		let bob_total_balance = total_balance!(BOB);
		let bob_token_balance =
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, BOB);

		let fulfillment_amount = 5;
		let residual = alice_token_balance / 10 - fulfillment_amount;
		assert_ok!(Marketplace::fulfill_order(
			RuntimeOrigin::signed(BOB),
			alice_order,
			fulfillment_amount
		));
		System::assert_has_event(RuntimeEvent::Marketplace(pallet_marketplace::Event::<
			Test,
			pallet_marketplace::Instance1,
		>::OrderPartiallyFulfilled {
			id: alice_order,
			who: BOB,
			amount: 5,
			residual,
		}));
		assert_eq!(total_balance!(BOB), bob_total_balance - 5 * alice_order_info.price_per_token);
		assert_eq!(total_balance!(ALICE), alice_total_balance);
		assert_eq!(
			<Test as pallet_insurances::Config>::SecondaryMarketToken::balance(alice_token, BOB),
			bob_token_balance
		);

		assert_eq!(OrderSMTsAmount::<Test, pallet_marketplace::Instance1>::iter().count(), 1);

		assert_ok!(PayoutProcessor::feed_event(
			RuntimeOrigin::signed(ALICE),
			(InsuranceType::Cyclone, 1),
			vec![],
		));

		for i in 0..5 {
			System::assert_has_event(RuntimeEvent::Marketplace(
				pallet_marketplace::Event::OrderCanceled { id: i },
			));
		}

		assert_eq!(OrderBook::<Test, pallet_marketplace::Instance1>::iter().count(), 0);
		assert_eq!(OrderSMTsAmount::<Test, pallet_marketplace::Instance1>::iter().count(), 0);
	});
}

#[test]
fn feed_event_works() {
	new_test_ext().execute_with(|| {
		setup_testing_environment();

		let metadata = fulfill_metadata(None, None, None, None, None, None);

		let proposal =
			RuntimeCall::Dao(pallet_dao::Call::allocate_liquidity { metadata: metadata.clone() });
		assert_ok!(Dao::request_insurance(
			RuntimeOrigin::signed(BOB),
			metadata,
			Box::new(proposal.clone())
		));

		let proposal_hash = <Test as frame_system::Config>::Hashing::hash_of(&proposal);

		frame_system::Pallet::<Test>::set_block_number(1);
		assert_ok!(Dao::add_member(RuntimeOrigin::root(), ALICE));
		assert_ok!(Dao::vote(RuntimeOrigin::signed(ALICE), proposal_hash, 0, true));

		System::assert_has_event(RuntimeEvent::Dao(pallet_dao::Event::LiquidityProvisionVoted {
			who: ALICE,
			proposal_index: 0,
			decision: true,
		}));

		frame_system::Pallet::<Test>::set_block_number(5);
		<pallet_dao::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);
		System::assert_last_event(RuntimeEvent::Collective(pallet_collective::Event::Executed {
			result: Ok(()),
			proposal_hash,
		}));
		frame_system::Pallet::<Test>::set_block_number(10);
		<pallet_dao::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);

		assert_ok!(PayoutProcessor::feed_event(
			RuntimeOrigin::signed(ALICE),
			(InsuranceType::Cyclone, 1),
			vec![],
		));

		System::assert_has_event(RuntimeEvent::PayoutProcessor(
			pallet_payout_processor::Event::HandledInsuranceEvent {
				who: ALICE,
				event: InsuranceType::Cyclone,
				location: 1,
				insurance: None,
			},
		));
	});
}

#[test]
fn feed_event_for_particular_insurance_works() {
	new_test_ext().execute_with(|| {
		setup_testing_environment();

		let collection_id = 0;
		let first_insurance_id = 0;
		let second_insurance_id = 1;

		let first_metadata = fulfill_metadata(None, None, None, None, None, None);
		let second_metadata = fulfill_metadata(None, None, None, None, Some(11), None);

		let first_proposal = RuntimeCall::Dao(pallet_dao::Call::allocate_liquidity {
			metadata: first_metadata.clone(),
		});
		let second_proposal = RuntimeCall::Dao(pallet_dao::Call::allocate_liquidity {
			metadata: second_metadata.clone(),
		});
		assert_ok!(Dao::request_insurance(
			RuntimeOrigin::signed(BOB),
			first_metadata.clone(),
			Box::new(first_proposal.clone())
		));
		assert_ok!(Dao::request_insurance(
			RuntimeOrigin::signed(BOB),
			second_metadata.clone(),
			Box::new(second_proposal.clone())
		));

		let first_proposal_hash = <Test as frame_system::Config>::Hashing::hash_of(&first_proposal);
		let second_proposal_hash =
			<Test as frame_system::Config>::Hashing::hash_of(&second_proposal);

		frame_system::Pallet::<Test>::set_block_number(1);
		assert_ok!(Dao::add_member(RuntimeOrigin::root(), ALICE));
		assert_ok!(Dao::vote(RuntimeOrigin::signed(ALICE), first_proposal_hash, 0, true));
		assert_ok!(Dao::vote(RuntimeOrigin::signed(ALICE), second_proposal_hash, 1, true));

		frame_system::Pallet::<Test>::set_block_number(5);
		<pallet_dao::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);
		System::assert_has_event(RuntimeEvent::Collective(pallet_collective::Event::Executed {
			result: Ok(()),
			proposal_hash: first_proposal_hash,
		}));
		System::assert_has_event(RuntimeEvent::Collective(pallet_collective::Event::Executed {
			result: Ok(()),
			proposal_hash: second_proposal_hash,
		}));
		frame_system::Pallet::<Test>::set_block_number(11);
		<pallet_payout_processor::Pallet<Test, _> as Hooks<
			<Test as frame_system::Config>::BlockNumber,
		>>::on_finalize(System::block_number());
		System::assert_has_event(RuntimeEvent::PayoutProcessor(crate::Event::<
			Test,
			crate::Instance1,
		>::InsuranceActivated {
			collection_id: collection_id.clone(),
			insurance_id: first_insurance_id.clone(),
			metadata: first_metadata.clone(),
		}));
		System::assert_has_event(RuntimeEvent::PayoutProcessor(crate::Event::<
			Test,
			crate::Instance1,
		>::InsuranceActivated {
			collection_id: collection_id.clone(),
			insurance_id: second_insurance_id.clone(),
			metadata: second_metadata.clone(),
		}));

		assert_ok!(PayoutProcessor::feed_event(
			RuntimeOrigin::signed(ALICE),
			(InsuranceType::Cyclone, 1_u8),
			vec![
				Some((collection_id.clone(), first_insurance_id.clone())),
				Some((collection_id.clone(), second_insurance_id.clone()))
			],
		));

		System::assert_has_event(RuntimeEvent::PayoutProcessor(
			pallet_payout_processor::Event::HandledInsuranceEvent {
				who: ALICE,
				event: first_metadata.name.clone(),
				location: first_metadata.location,
				insurance: Some((collection_id.clone(), first_insurance_id.clone())),
			},
		));
		System::assert_has_event(RuntimeEvent::PayoutProcessor(
			pallet_payout_processor::Event::HandledInsuranceEvent {
				who: ALICE,
				event: second_metadata.name.clone(),
				location: second_metadata.location,
				insurance: Some((collection_id.clone(), second_insurance_id.clone())),
			},
		));

		System::assert_has_event(RuntimeEvent::PayoutProcessor(
			pallet_payout_processor::Event::PaidOutInsurance {
				beneficiary: first_metadata.creator.clone(),
				event: first_metadata.name.clone(),
				collection_id,
				insurance_id: first_insurance_id,
				metadata: InsuranceMetadata {
					name: first_metadata.name,
					location: first_metadata.location,
					creator: first_metadata.creator,
					status: InsuranceStatus::PaidOut,
					underwrite_amount: first_metadata.underwrite_amount,
					premium_amount: first_metadata.premium_amount,
					contract_link: first_metadata.contract_link,
					starts_on: first_metadata.starts_on,
					ends_on: first_metadata.ends_on,
					smt_id: None,
				},
			},
		));
		System::assert_has_event(RuntimeEvent::PayoutProcessor(
			pallet_payout_processor::Event::PaidOutInsurance {
				beneficiary: second_metadata.creator.clone(),
				event: second_metadata.name.clone(),
				collection_id,
				insurance_id: second_insurance_id,
				metadata: InsuranceMetadata {
					name: second_metadata.name,
					location: second_metadata.location,
					creator: second_metadata.creator,
					status: InsuranceStatus::PaidOut,
					underwrite_amount: second_metadata.underwrite_amount,
					premium_amount: second_metadata.premium_amount,
					contract_link: second_metadata.contract_link,
					starts_on: second_metadata.starts_on,
					ends_on: second_metadata.ends_on,
					smt_id: None,
				},
			},
		));
	});
}

#[test]
fn feed_event_fails_with_wrong_data() {
	new_test_ext().execute_with(|| {
		setup_testing_environment();

		let collection_id = 0;
		let insurance_id = 0;

		let metadata = fulfill_metadata(None, None, None, None, None, None);

		let proposal =
			RuntimeCall::Dao(pallet_dao::Call::allocate_liquidity { metadata: metadata.clone() });
		assert_ok!(Dao::request_insurance(
			RuntimeOrigin::signed(BOB),
			metadata.clone(),
			Box::new(proposal.clone())
		));

		let proposal_hash = <Test as frame_system::Config>::Hashing::hash_of(&proposal);

		frame_system::Pallet::<Test>::set_block_number(1);
		assert_ok!(Dao::add_member(RuntimeOrigin::root(), ALICE));
		assert_ok!(Dao::vote(RuntimeOrigin::signed(ALICE), proposal_hash, 0, true));

		System::assert_has_event(RuntimeEvent::Dao(pallet_dao::Event::LiquidityProvisionVoted {
			who: ALICE,
			proposal_index: 0,
			decision: true,
		}));

		frame_system::Pallet::<Test>::set_block_number(5);
		<pallet_dao::Pallet<Test, _> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);
		System::assert_last_event(RuntimeEvent::Collective(pallet_collective::Event::Executed {
			result: Ok(()),
			proposal_hash,
		}));
		frame_system::Pallet::<Test>::set_block_number(10);
		<pallet_payout_processor::Pallet<Test, _> as Hooks<
			<Test as frame_system::Config>::BlockNumber,
		>>::on_finalize(System::block_number());
		System::assert_has_event(RuntimeEvent::PayoutProcessor(crate::Event::<
			Test,
			crate::Instance1,
		>::InsuranceActivated {
			collection_id: collection_id.clone(),
			insurance_id: insurance_id.clone(),
			metadata: metadata.clone(),
		}));

		// Error because we create only one insurance
		// with collection_id = 0, insurance_id = 0
		assert_err!(
			PayoutProcessor::feed_event(
				RuntimeOrigin::signed(ALICE),
				(metadata.name.clone(), metadata.location.clone()),
				vec![Some((1, 2))]
			),
			pallet_dao::Error::<Test, _>::NoMetadataFound
		);

		// Error with location = 5, but we have location 1
		assert_err!(
			PayoutProcessor::feed_event(
				RuntimeOrigin::signed(ALICE),
				(metadata.name.clone(), 5),
				vec![Some((collection_id, insurance_id))]
			),
			pallet_payout_processor::Error::<Test, _>::InvalidInsuranceMetadata
		);
	});
}
