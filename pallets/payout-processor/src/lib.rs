#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod types;
use types::InsuranceType;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungibles::{Mutate, Unbalanced},
			tokens::{Fortitude, Precision, Preservation},
			Currency, ReservableCurrency, SortedMembers,
		},
	};
	use frame_system::{
		ensure_root, ensure_signed,
		offchain::{AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer},
		pallet_prelude::*,
	};
	use scale_info::prelude::format;
	use sp_core::crypto::KeyTypeId;
	use sp_runtime::{
		offchain::{http, Duration},
		traits::Zero,
		Saturating,
	};
	use sp_std::prelude::*;

	use crate::types::InsuranceReason;
	use offchain_utils::offchain_api_key::OffchainApiKey;
	use pallet_dao::traits::ProvideAccountId as ProvideDaoAccountId;
	use pallet_insurances::types::InsuranceStatus;
	use serde_json::Value;

	pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"wapi");
	pub const WEATHER_API_ENDPOINT: &str = "https://api.open-meteo.com/v1/forecast";
	pub const DEFAULT_EVENT_THRESHOLD: u32 = 50; // 50mm of rain as default threshold

	pub struct CustomApiKeyFetcher;

	impl OffchainApiKey for CustomApiKeyFetcher {}

	pub mod crypto {
		use super::KEY_TYPE;
		use sp_core::sr25519::Signature as Sr25519Signature;
		use sp_runtime::{
			app_crypto::{app_crypto, sr25519},
			traits::Verify,
			MultiSignature, MultiSigner,
		};

		app_crypto!(sr25519, KEY_TYPE);

		pub struct AuthId;

		impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for AuthId {
			type RuntimeAppPublic = Public;
			type GenericSignature = sp_core::sr25519::Signature;
			type GenericPublic = sp_core::sr25519::Public;
		}

		impl
			frame_system::offchain::AppCrypto<
				<Sr25519Signature as Verify>::Signer,
				Sr25519Signature,
			> for AuthId
		{
			type RuntimeAppPublic = Public;
			type GenericSignature = sp_core::sr25519::Signature;
			type GenericPublic = sp_core::sr25519::Public;
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::config]
	pub trait Config<I: 'static = ()>:
		frame_system::Config
		+ orml_oracle::Config<
			I,
			OracleKey = (InsuranceType, u8),
			OracleValue = Option<(
				<Self as pallet_insurances::Config>::NftId,
				<Self as pallet_insurances::Config>::NftId,
			)>,
		> + pallet_insurances::Config
		+ pallet_dao::pallet::Config<I>
		+ pallet_marketplace::Config<I>
		+ CreateSignedTransaction<Call<Self, I>>
	{
		/// The identifier type for an offchain worker.
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;

		type RuntimeEvent: From<Event<Self, I>>
			+ IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type Currency: Currency<Self::AccountId, Balance = <Self as pallet_insurances::Config>::Balance>
			+ ReservableCurrency<Self::AccountId>;

		type DaoAccountIdProvider: ProvideDaoAccountId<Self::AccountId>;

		#[pallet::constant]
		type BaseSecondaryMarketTokenPrice: Get<<Self as pallet_insurances::Config>::Balance>;

		type WeightInfo: WeightInfo;
	}

	/// This storage contains the latest known total number of active insurances across all
	/// users/collections.
	///
	/// It is updated on every scan of the [`pallet_insurances::Metadata`], which happens at the end
	/// of each block and during the call to [`feed_event`] extrinsic. The only case the value in
	/// this storage will differ from real one is when `pallet_dao`'s `on_finalize` hook is executed
	/// before `on_finalize` of this pallet, though, the error will be less than the max amount of
	/// insurances that can be submitted per block, so we can safely ignore it. As a result, it
	/// gives us a pretty good estimate of how many insurances are present in the storage at the
	/// moment, so we can use it to generate weight for [`feed_event`] extrinsic.
	///
	/// [`feed_event`]: Pallet::feed_event
	#[pallet::storage]
	#[pallet::getter(fn insurance_count)]
	pub type InsuranceCount<T: Config<I>, I: 'static = ()> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn event_thresholds)]
	pub type EventThresholds<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		Blake2_128Concat,
		InsuranceType,
		u32, // Threshold in mm (integer for simplicity)
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn location_coordinates)]
	pub type LocationCoordinates<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		Blake2_128Concat,
		u8,         // Location ID
		(i32, i32), // Latitude and longitude multiplied by 10000 to store as integers
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn location_names)]
	pub type LocationNames<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		Blake2_128Concat,
		u8,                           // Location ID
		BoundedVec<u8, ConstU32<64>>, // Location name as bounded bytes
		OptionQuery,
	>;

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			<T as Config<I>>::WeightInfo::do_execute_insurance_payout()
		}

		fn on_finalize(n: BlockNumberFor<T>) {
			Self::do_execute_insurance_payout(n)
		}

		fn offchain_worker(block_number: BlockNumberFor<T>) {
			log::info!(
				"Payout processor::ocw Offchain worker started at block: {:?}",
				block_number
			);

			// Run every 100 blocks
			if block_number % 100u32.into() != 0u32.into() {
				return;
			}

			// Specify which AuthorityId we want to use
			let signer = Signer::<T, <T as Config<I>>::AuthorityId>::all_accounts();
			if !signer.can_sign() {
				log::error!("No accounts available for signing");
				return;
			}

			match Self::fetch_weather_data() {
				Ok(events) => {
					if events.is_empty() {
						log::info!("No weather events detected");
						return;
					}
					for (event_type, location) in events {
						if let Err(e) = Self::submit_weather_event(event_type, location) {
							log::error!(
								"Failed to submit weather event for location {}: {:?}",
								location,
								e
							);
						}
					}
				},
				Err(e) => {
					log::error!("Error fetching weather data: {:?}", e);
				},
			}
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// Processed insured event
		HandledInsuranceEvent {
			/// Event feeder
			who: T::AccountId,
			/// Event type
			event: InsuranceType,
			/// Location of event
			location: u8,
			/// Insurance
			/// - 1st `T::NftId` represents the collection_id (1 collection per user)
			/// - 2nd `T::NftId` represents the insurance_id (unique among in each collection)
			insurance: Option<(T::NftId, T::NftId)>,
		},

		/// Insurance paid out
		PaidOutInsurance {
			/// Insured user
			beneficiary: T::AccountId,
			/// Event type
			event: InsuranceType,
			/// Insurance collection id
			collection_id: T::NftId,
			/// Insurance id
			insurance_id: T::NftId,
			/// Insurance metadata
			metadata: pallet_insurances::InsuranceMetadataOf<T>,
		},

		/// Insurance activated
		InsuranceActivated {
			/// Insurance collection id
			collection_id: T::NftId,
			/// Insurance id
			insurance_id: T::NftId,
			/// Insurance metadata
			metadata: pallet_insurances::InsuranceMetadataOf<T>,
		},

		/// Insurance expired
		InsuranceExpired {
			/// Insurance collection id
			collection_id: T::NftId,
			/// Insurance id
			insurance_id: T::NftId,
			/// Insurance metadata
			metadata: pallet_insurances::InsuranceMetadataOf<T>,
		},

		/// Liquidity provider redeemed their tokens
		PremiumPaidOut {
			/// Secondary market token id
			token_id: <T as pallet_insurances::Config>::AssetId,
			/// Redeemer
			beneficiary: T::AccountId,
		},

		/// Insurance has SMT and it's expired OR event_occurred
		InsuranceAwaitingPremiumClaim {
			/// Insurance collection id
			collection_id: T::NftId,
			/// Insurance id
			insurance_id: T::NftId,
			/// Reason
			reason: InsuranceReason,
		},

		/// Liquidity provider redeemed last tokens with this token id and destroyed the asset
		SecondaryMarketTokenDestroyed {
			/// Secondary market token id
			token_id: <T as pallet_insurances::Config>::AssetId,
		},

		OracleMemberAdded {
			account_id: T::AccountId,
		},
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Account does not have enough funds
		NotEnoughFunds,

		/// Could not find token id
		UnknownTokenId,

		/// Can't claim premium until the insurance is expired
		InsuranceNotExpired,

		/// The account trying to claim premium payout does not
		/// have SMTs on their balance
		SecondaryMarketTokenBalanceIsZero,

		/// Invalid event or/and location argument for `feed_event` function
		InvalidInsuranceMetadata,

		NotOracleMember,

		AlreadyOracleMember,

		/// Location name is too long
		LocationNameTooLong,
	}

	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Handles occurred event.
		///
		/// Called in `T::OnNewData` when some external event occurs.
		///
		/// The function handles events for a specific `event` and `location`, as the parameters
		/// names suggest. Optionally, the `insurance` parameter can be added for when an event
		/// occurs for a specific insurance that relates to the same `event` / `location` combo.
		/// If no value for `insurance` is provided, all related insurances will be confirmed.

		pub fn handle_insurance_event(
			event_feeder: &T::AccountId,
			event: &InsuranceType,
			location: &u8,
			insurance: Option<(T::NftId, T::NftId)>,
		) {
			let mut total_insurances = 0;
			let mut removed_insurances = 0;
			pallet_insurances::Metadata::<T>::translate(
				|collection_id,
				 insurance_id,
				 mut metadata: pallet_insurances::InsuranceMetadataOf<T>| {
					use pallet_insurances::types::InsuranceStatus;
					total_insurances += 1;

					let (collection_id, insurance_id) = match insurance.clone() {
						Some((certain_collection_id, certain_insurance_id)) =>
							if certain_collection_id == collection_id &&
								certain_insurance_id == insurance_id
							{
								(certain_collection_id, certain_insurance_id)
							} else {
								return Some(metadata);
							},
						None => (collection_id, insurance_id),
					};

					if metadata.status == InsuranceStatus::NotStarted &&
						metadata.starts_on <= frame_system::Pallet::<T>::block_number()
					{
						metadata.status = InsuranceStatus::Active;
					}

					if metadata.status != InsuranceStatus::Active ||
						metadata.name != *event ||
						metadata.location != *location
					{
						return Some(metadata);
					}

					let dao_account_id =
						<T as pallet::Config<I>>::DaoAccountIdProvider::account_id();
					// if <T as pallet_insurances::Config>::StableCurrency::reserved_balance(&
					// metadata.creator) < 	metadata.premium_amount
					// {
					// 	return Some(metadata)
					// }
					// if <T as pallet_insurances::Config>::StableCurrency::reserved_balance(&
					// dao_account_id) < 	metadata.underwrite_amount
					// {
					// 	return Some(metadata)
					// }

					if let Err(e) = pallet_insurances::Pallet::<T>::destroy_insurance(
						metadata.creator.clone(),
						collection_id.clone(),
						insurance_id.clone(),
					) {
						log::warn!(
							target: "runtime::payout-processor",
							"Failed to destroy insurance {:?} from collection {:?}. {:?}",
							collection_id,
							insurance_id,
							e
						);
						return Some(metadata);
					};

					<T as pallet_insurances::Config>::StableCurrency::increase_balance(
						<T as pallet_insurances::Config>::UsdtId::get(),
						&metadata.creator,
						metadata.premium_amount,
						Precision::Exact,
					)
					.ok()?;

					// Unreserve underwrite_amount - premium_amount to use this later
					<T as pallet_insurances::Config>::StableCurrency::increase_balance(
						<T as pallet_insurances::Config>::UsdtId::get(),
						&dao_account_id,
						metadata.underwrite_amount.saturating_sub(metadata.premium_amount),
						Precision::Exact,
					)
					.ok()?;

					// Transfer only underwrite_amount - premium_amount simulating a transfer
					// premium_amount from creator to dao_account, then transfer underwrite_amount
					// from dao_account to creator
					<T as pallet_insurances::Config>::StableCurrency::transfer(
						<T as pallet_insurances::Config>::UsdtId::get(),
						&dao_account_id,
						&metadata.creator,
						metadata.underwrite_amount.saturating_sub(metadata.premium_amount),
						Preservation::Preserve,
					)
					.expect("Unreachable: failed to payout user");

					metadata.status = InsuranceStatus::PaidOut;
					Pallet::<T, I>::deposit_event(Event::<T, I>::PaidOutInsurance {
						beneficiary: metadata.creator.clone(),
						event: event.clone(),
						collection_id: collection_id.clone(),
						insurance_id: insurance_id.clone(),
						metadata: metadata.clone(),
					});

					if metadata.smt_id.is_some() {
						if let Err(e) = pallet_marketplace::Pallet::<T, I>::do_clean_orders(
							metadata.smt_id.clone().unwrap(),
						) {
							log::warn!(
								target: "runtime::payout-processor",
								"Failed to remove orders for token {:?}. {:?}",
								metadata.smt_id,
								e
							);
							return Some(metadata);
						}
						metadata.status = InsuranceStatus::PremiumPayoutPending;
						Self::deposit_event(Event::<T, I>::InsuranceAwaitingPremiumClaim {
							collection_id,
							insurance_id,
							reason: InsuranceReason::EventOccurred,
						});
						return Some(metadata);
					} else {
						<T as pallet_insurances::Config>::StableCurrency::increase_balance(
							<T as pallet_insurances::Config>::UsdtId::get(),
							&dao_account_id,
							metadata.premium_amount,
							Precision::Exact,
						)
						.ok()?;

						pallet_dao::pallet::Pallet::<T, I>::do_claim_dao_profits(
							collection_id.clone(),
							insurance_id.clone(),
						)
						.expect("Unreachable: failed to claim dao profits");

						pallet_insurances::Metadata::<T>::remove(collection_id, insurance_id);
					}

					removed_insurances += 1;
					None
				},
			);

			InsuranceCount::<T, I>::set(total_insurances - removed_insurances);
			Pallet::<T, I>::deposit_event(Event::<T, I>::HandledInsuranceEvent {
				who: event_feeder.clone(),
				event: event.clone(),
				location: *location,
				insurance,
			});
		}
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I>
	where
		T: Config<
			I,
			OracleKey = (InsuranceType, u8), // u8 represents the location
			OracleValue = Option<(
				<T as pallet_insurances::Config>::NftId,
				<T as pallet_insurances::Config>::NftId,
			)>,
		>,
	{
		/// Feed an external event.
		///
		/// Requires an authorized operator.
		///
		/// The `event_info` parameter is `T::OracleKey` because this extrinsic does not dispatch
		/// the event. The real dispatch occurs in `T::OnNewData` and all the type constraints
		/// should be specified there.

		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config<I>>::WeightInfo::feed_event(InsuranceCount::<T, I>::get()))]
		pub fn feed_event(
			origin: OriginFor<T>,
			event_info: T::OracleKey,
			insurances: Vec<T::OracleValue>,
		) -> DispatchResult {
			use orml_oracle::DataFeeder;

			let mut insurances_for_event = Vec::new();
			let feeder: T::AccountId = ensure_signed(origin.clone()).or_else(|_| {
				ensure_root(origin.clone())
					.map(|_| <T as orml_oracle::Config<I>>::RootOperatorAccountId::get())
			})?;
			let (event, location) = event_info;

			// if `insurances` is empty, confirm peril event for all insurances that use the
			// `event_info` provided
			if insurances.is_empty() {
				orml_oracle::Pallet::<T, I>::feed_value(feeder, (event, location), None)?;
				return Ok(());
			}
			// if `insurances` is NOT empty, confirm peril event ONLY for the `insurances`
			// provided
			for (collection_id, insurance_id) in insurances.into_iter().flatten() {
				let metadata = pallet_insurances::Metadata::<T>::get(
					collection_id.clone(),
					insurance_id.clone(),
				)
				.ok_or(pallet_dao::pallet::Error::<T, I>::NoMetadataFound)?;

				ensure!(
					metadata.name == event && metadata.location == location,
					Error::<T, I>::InvalidInsuranceMetadata
				);

				insurances_for_event
					.push(((event.clone(), location), Some((collection_id, insurance_id))));
			}

			// feed value for specific insurance
			orml_oracle::Pallet::<T, I>::feed_values(origin, insurances_for_event)
				.map_err(|e| e.error)?;
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config<I>>::WeightInfo::claim_premium_payout())]
		#[frame_support::transactional]
		pub fn claim_premium_payout(
			origin: OriginFor<T>,
			token_id: <T as pallet_insurances::Config>::AssetId,
		) -> DispatchResult {
			use frame_support::traits::tokens::fungibles::{Inspect, Mutate};
			let beneficiary = ensure_signed(origin)?;

			let (collection_id, insurance_id) =
				pallet_insurances::SmtIdToInsurance::<T>::get(token_id.clone())
					.ok_or(Error::<T, I>::UnknownTokenId)?;
			let metadata =
				pallet_insurances::Metadata::<T>::get(collection_id.clone(), insurance_id.clone())
					.unwrap();

			ensure!(
				metadata.status == pallet_insurances::types::InsuranceStatus::PayoutPending ||
					metadata.status ==
						pallet_insurances::types::InsuranceStatus::PremiumPayoutPending,
				Error::<T, I>::InsuranceNotExpired
			);

			let secondary_market_token_balance =
				<T as pallet_insurances::Config>::SecondaryMarketToken::balance(
					token_id.clone(),
					&beneficiary,
				);

			ensure!(
				secondary_market_token_balance !=
					<<T as pallet_insurances::Config>::SecondaryMarketToken as Inspect<
						<T as frame_system::Config>::AccountId,
					>>::Balance::zero(),
				Error::<T, I>::SecondaryMarketTokenBalanceIsZero
			);

			let initial_mint = metadata.underwrite_amount /
				<T as pallet::Config<I>>::BaseSecondaryMarketTokenPrice::get();
			let claimed_underwrite_amount =
				metadata.underwrite_amount / initial_mint * secondary_market_token_balance;
			let claimed_premium_amount = metadata.premium_amount * 1_000_u32.into() / initial_mint *
				secondary_market_token_balance /
				1_000_u32.into();
			let dao_account_id = <T as pallet::Config<I>>::DaoAccountIdProvider::account_id();

			match metadata.status {
				InsuranceStatus::PayoutPending => {
					<T as pallet_insurances::Config>::StableCurrency::increase_balance(
						<T as pallet_insurances::Config>::UsdtId::get(),
						&dao_account_id,
						claimed_underwrite_amount + claimed_premium_amount,
						Precision::Exact,
					)?;
					<T as pallet_insurances::Config>::StableCurrency::transfer(
						<T as pallet_insurances::Config>::UsdtId::get(),
						&dao_account_id,
						&beneficiary,
						claimed_underwrite_amount + claimed_premium_amount,
						Preservation::Preserve,
					)
					.expect("Unreachable: failed to payout user");
				},
				InsuranceStatus::PremiumPayoutPending => {
					<T as pallet_insurances::Config>::StableCurrency::increase_balance(
						<T as pallet_insurances::Config>::UsdtId::get(),
						&dao_account_id,
						claimed_premium_amount,
						Precision::Exact,
					)?;

					<T as pallet_insurances::Config>::StableCurrency::transfer(
						<T as pallet_insurances::Config>::UsdtId::get(),
						&dao_account_id,
						&beneficiary,
						claimed_premium_amount,
						Preservation::Preserve,
					)
					.expect("Unreachable: failed to payout user");
				},
				_ => {},
			}

			<T as pallet_insurances::Config>::SecondaryMarketToken::burn_from(
				token_id.clone(),
				&beneficiary,
				secondary_market_token_balance,
				Precision::Exact,
				Fortitude::Polite,
			)?;
			Self::deposit_event(Event::<T, I>::PremiumPaidOut {
				token_id: token_id.clone(),
				beneficiary,
			});

			if <T as pallet_insurances::Config>::SecondaryMarketToken::total_issuance(
				token_id.clone(),
			) == 0u32.into()
			{
				pallet_insurances::SmtIdToInsurance::<T>::remove(token_id.clone());
				pallet_insurances::Metadata::<T>::remove(collection_id, insurance_id);
				Self::deposit_event(Event::<T, I>::SecondaryMarketTokenDestroyed { token_id });
			}

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config<I>>::WeightInfo::add_oracle_member())]
		pub fn add_oracle_member(origin: OriginFor<T>, new_member: T::AccountId) -> DispatchResult {
			use frame_support::traits::SortedMembers;

			ensure_root(origin)?;

			ensure!(
				Self::is_oracle_member(&new_member).is_err(),
				Error::<T, I>::AlreadyOracleMember
			);

			let members: Vec<T::AccountId> =
				<T as orml_oracle::Config<I>>::Members::sorted_members();

			let mut new_members = members;
			new_members.push(new_member.clone());

			<T as orml_oracle::Config<I>>::Members::sorted_members();

			Self::deposit_event(Event::<T, I>::OracleMemberAdded { account_id: new_member });

			Ok(())
		}

		/// Set event threshold for an insurance type
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config<I>>::WeightInfo::set_event_threshold())]
		pub fn set_event_threshold(
			origin: OriginFor<T>,
			insurance_type: InsuranceType,
			threshold_value: u32,
		) -> DispatchResult {
			let feeder: T::AccountId = ensure_signed(origin.clone()).or_else(|_| {
				ensure_root(origin.clone())
					.map(|_| <T as orml_oracle::Config<I>>::RootOperatorAccountId::get())
			})?;

			// Ensure the caller is an oracle member
			Self::is_oracle_member(&feeder)?;

			EventThresholds::<T, I>::insert(insurance_type.clone(), threshold_value);

			log::info!(
				"Event threshold for {:?} set to {} by {:?}",
				insurance_type,
				threshold_value,
				feeder
			);

			Ok(())
		}

		/// Set coordinates for a location
		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config<I>>::WeightInfo::set_location_coordinates())]
		pub fn set_location_coordinates(
			origin: OriginFor<T>,
			location_id: u8,
			latitude: i32,  // Latitude multiplied by 10000 (e.g., 12.3456 -> 123456)
			longitude: i32, // Longitude multiplied by 10000
		) -> DispatchResult {
			let feeder: T::AccountId = ensure_signed(origin.clone()).or_else(|_| {
				ensure_root(origin.clone())
					.map(|_| <T as orml_oracle::Config<I>>::RootOperatorAccountId::get())
			})?;

			// Ensure the caller is an oracle member
			Self::is_oracle_member(&feeder)?;

			LocationCoordinates::<T, I>::insert(location_id, (latitude, longitude));

			log::info!(
				"Coordinates for location {} set to {}.{}, {}.{} by {:?}",
				location_id,
				latitude / 10000,
				latitude.abs() % 10000,
				longitude / 10000,
				longitude.abs() % 10000,
				feeder
			);

			Ok(())
		}

		/// Set name for a location
		#[pallet::call_index(5)]
		#[pallet::weight(<T as Config<I>>::WeightInfo::set_location_name())]
		pub fn set_location_name(
			origin: OriginFor<T>,
			location_id: u8,
			name: Vec<u8>,
		) -> DispatchResult {
			let feeder: T::AccountId = ensure_signed(origin.clone()).or_else(|_| {
				ensure_root(origin.clone())
					.map(|_| <T as orml_oracle::Config<I>>::RootOperatorAccountId::get())
			})?;

			// Ensure the caller is an oracle member
			Self::is_oracle_member(&feeder)?;

			let bounded_name = BoundedVec::<u8, ConstU32<64>>::try_from(name)
				.map_err(|_| Error::<T, I>::LocationNameTooLong)?;

			LocationNames::<T, I>::insert(location_id, bounded_name.clone());

			if let Ok(name_str) = sp_std::str::from_utf8(&bounded_name) {
				log::info!("Name for location {} set to {} by {:?}", location_id, name_str, feeder);
			}

			Ok(())
		}
	}

	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		pub fn is_oracle_member(account_id: &T::AccountId) -> DispatchResult {
			if !<T as orml_oracle::Config<I>>::Members::contains(account_id) {
				return Err(Error::<T, I>::NotOracleMember.into());
			}
			Ok(())
		}

		pub fn do_execute_insurance_payout(n: BlockNumberFor<T>) {
			let mut total_insurances = 0;
			let mut removed_insurances = 0;
			pallet_insurances::Metadata::<T>::translate(
				|collection_id,
				 insurance_id,
				 mut metadata: pallet_insurances::InsuranceMetadataOf<T>| {
					use pallet_insurances::types::InsuranceStatus;
					total_insurances += 1;

					if metadata.status == InsuranceStatus::NotStarted && n >= metadata.starts_on {
						metadata.status = InsuranceStatus::Active;
						Self::deposit_event(Event::InsuranceActivated {
							collection_id: collection_id.clone(),
							insurance_id: insurance_id.clone(),
							metadata: metadata.clone(),
						});
					}

					if metadata.ends_on > n {
						return Some(metadata);
					}

					let dao_account_id =
						<T as pallet::Config<I>>::DaoAccountIdProvider::account_id();
					// let creator_reserved_balance =
					// 	<T as pallet_insurances::Config>::StableCurrency::reserved_balance(&
					// metadata. creator); if creator_reserved_balance <
					// metadata.premium_amount { 	#[cfg(not(feature = "runtime-benchmarks"))]
					// 	log::warn!(
					// 		target: "runtime::payout-processor",
					// 		"Insurance holder does not have expected reserved balance. Expected at
					// least: {:?} Current balance: {:?}", 		metadata.premium_amount,
					// 		creator_reserved_balance
					// 	);
					// 	return Some(metadata)
					// }

					// let dao_reserved_balance =
					// 	<T as pallet_insurances::Config>::Currency::reserved_balance(&
					// dao_account_id); if dao_reserved_balance < metadata.underwrite_amount {
					// 	#[cfg(not(feature = "runtime-benchmarks"))]
					// 	log::warn!(
					// 		target: "runtime::payout-processor",
					// 		"DAO does not have expected reserved balance. Expected at least: {:?}
					// Current balance: {:?}", 		metadata.underwrite_amount,
					// 		dao_reserved_balance,
					// 	);
					// 	return Some(metadata)
					// }

					if let Err(e) = pallet_insurances::Pallet::<T>::destroy_insurance(
						metadata.creator.clone(),
						collection_id.clone(),
						insurance_id.clone(),
					) {
						log::warn!(
							target: "runtime::payout-processor",
							"Failed to destroy insurance {:?} from collection {:?}. {:?}",
							insurance_id,
							collection_id,
							e
						);
						return Some(metadata);
					};

					<T as pallet_insurances::Config>::StableCurrency::increase_balance(
						<T as pallet_insurances::Config>::UsdtId::get(),
						&metadata.creator,
						metadata.premium_amount,
						Precision::Exact,
					)
					.ok()?;
					<T as pallet_insurances::Config>::StableCurrency::transfer(
						<T as pallet_insurances::Config>::UsdtId::get(),
						&metadata.creator,
						&dao_account_id,
						metadata.premium_amount,
						Preservation::Preserve,
					)
					.expect("Unreachable: failed to transfer user premium on insurance expiration");

					metadata.status = InsuranceStatus::Expired;
					Self::deposit_event(Event::InsuranceExpired {
						collection_id: collection_id.clone(),
						insurance_id: insurance_id.clone(),
						metadata: metadata.clone(),
					});

					if metadata.smt_id.is_some() {
						if let Err(e) = pallet_marketplace::Pallet::<T, I>::do_clean_orders(
							metadata.smt_id.clone().unwrap(),
						) {
							log::warn!(
								target: "runtime::payout-processor",
								"Failed to remove orders for token {:?}. {:?}",
								metadata.smt_id,
								e
							);
							return Some(metadata);
						}
						<T as pallet_insurances::Config>::StableCurrency::decrease_balance(
							T::UsdtId::get(),
							&dao_account_id,
							metadata.premium_amount,
							Precision::Exact,
							Preservation::Preserve,
							Fortitude::Polite,
						).expect("Unreachable: failed to reserve premium amount for secondary market payouts");
						metadata.status = InsuranceStatus::PayoutPending;
						Self::deposit_event(Event::<T, I>::InsuranceAwaitingPremiumClaim {
							collection_id,
							insurance_id,
							reason: InsuranceReason::Expired,
						});
						return Some(metadata);
					} else {
						<T as pallet_insurances::Config>::StableCurrency::increase_balance(
							<T as pallet_insurances::Config>::UsdtId::get(),
							&dao_account_id,
							metadata.underwrite_amount,
							Precision::Exact,
						)
						.ok()?;

						pallet_dao::pallet::Pallet::<T, I>::do_claim_dao_profits(
							collection_id.clone(),
							insurance_id.clone(),
						)
						.expect("Unreachable: failed to claim dao profits");

						pallet_insurances::Metadata::<T>::remove(collection_id, insurance_id);
					}

					removed_insurances += 1;
					None
				},
			);

			InsuranceCount::<T, I>::set(total_insurances - removed_insurances);
		}

		// Modify the fetch_weather_data function to check all active insurances
		fn fetch_weather_data() -> Result<Vec<(InsuranceType, u8)>, http::Error> {
			log::info!("Payout processor::ocw Fetching weather data");

			let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(5_000));
			let mut weather_events = Vec::new();

			// Get all active insurances grouped by location
			let mut locations = Vec::new();
			let mut seen_locations = sp_std::collections::btree_set::BTreeSet::new();
			let mut location_insurance_types: sp_std::collections::btree_map::BTreeMap<
				u8,
				sp_std::collections::btree_set::BTreeSet<InsuranceType>,
			> = sp_std::collections::btree_map::BTreeMap::new();

			// Iterate through all insurances to find active ones
			pallet_insurances::Metadata::<T>::iter().for_each(|(_, _, metadata)| {
				if metadata.status == pallet_insurances::types::InsuranceStatus::Active {
					// Only add each location once to the locations vector
					if seen_locations.insert(metadata.location) {
						locations.push(metadata.location);
					}

					// Add the insurance type to the set for this location
					location_insurance_types
						.entry(metadata.location)
						.or_insert_with(sp_std::collections::btree_set::BTreeSet::new)
						.insert(metadata.name.clone());

					log::info!(
						"Added insurance type {:?} for location ID: {}",
						metadata.name,
						metadata.location
					);
				}
			});

			if locations.is_empty() {
				log::info!("No active insurance locations found");
				return Ok(weather_events);
			}

			log::info!("Found {} active insurance locations", locations.len());

			// Process each unique location
			for location_id in locations {
				// Get coordinates for this location
				let (latitude, longitude) = match Self::get_coordinates_for_location(location_id) {
					Some(coords) => coords,
					None => {
						log::warn!("Invalid location ID: {}", location_id);
						continue;
					},
				};

				let insurance_types = match location_insurance_types.get(&location_id) {
					Some(types) => {
						if types.is_empty() {
							log::warn!(
								"Empty insurance types set for location ID: {}",
								location_id
							);
							continue;
						}
						types
					},
					None => {
						log::warn!("No insurance types found for location ID: {}", location_id);
						continue;
					},
				};

				// Construct the URL with parameters for Open-Meteo API
				let url = format!(
					"{}?latitude={}&longitude={}&daily=precipitation_sum&current=precipitation&timezone=GMT&forecast_days=1",
					WEATHER_API_ENDPOINT, latitude, longitude
				);

				log::info!(
					"Fetching weather data for {} (ID: {}): {}",
					Self::get_location_name(location_id),
					location_id,
					url
				);

				// Make the request
				let request = http::Request::get(&url);
				let pending =
					request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;

				let response =
					pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;

				if response.code != 200 {
					log::warn!(
						"Unexpected status code: {} for location {}",
						response.code,
						location_id
					);
					continue;
				}

				let body = response.body().collect::<Vec<u8>>();

				// Parse the response and extract weather events if thresholds are exceeded
				let triggered_events =
					Self::parse_open_meteo_response(&body, location_id, insurance_types);
				weather_events.extend(triggered_events);
			}

			Ok(weather_events)
		}

		// Get event threshold for an insurance type from storage or use default
		fn get_event_threshold(insurance_type: &InsuranceType) -> u32 {
			let threshold = EventThresholds::<T, I>::get(insurance_type);
			if threshold == 0 {
				DEFAULT_EVENT_THRESHOLD
			} else {
				threshold
			}
		}

		// Get coordinates for a location from storage or use hardcoded fallback
		fn get_coordinates_for_location(location_id: u8) -> Option<(f64, f64)> {
			if let Some((lat, long)) = LocationCoordinates::<T, I>::get(location_id) {
				// Convert from integer representation (multiplied by 10000) back to float
				return Some((lat as f64 / 10000.0, long as f64 / 10000.0));
			}

			// Fallback to hardcoded values if not in storage
			match location_id {
				0 => Some((27.6648, -81.5158)),   // Florida, USA
				1 => Some((12.8797, 121.7740)),   // Philippines
				2 => Some((36.2048, 138.2529)),   // Japan
				3 => Some((21.5218, -77.7812)),   // Cuba
				4 => Some((25.0343, -77.3963)),   // Bahamas
				5 => Some((-18.7669, 46.8691)),   // Madagascar
				6 => Some((23.6345, -102.5528)),  // Mexico
				7 => Some((20.5937, 78.9629)),    // India
				8 => Some((-1.8312, -78.1834)),   // Ecuador
				9 => Some((-0.7893, 113.9213)),   // Indonesia
				10 => Some((36.7783, -119.4179)), // California, USA
				_ => None,
			}
		}

		// Get location name from storage or use hardcoded fallback
		fn get_location_name(location_id: u8) -> &'static str {
			if let Some(name) = LocationNames::<T, I>::get(location_id) {
				if let Ok(name_str) = sp_std::str::from_utf8(&name) {
					// This is unsafe because we're returning a static reference to a temporary
					// value. In a real implementation, you'd want to handle this differently
					let boxed: Box<str> = name_str.into();
					return Box::leak(boxed);
				}
			}

			// Fallback to hardcoded values if not in storage
			match location_id {
				0 => "Florida, USA",
				1 => "Philippines",
				2 => "Japan",
				3 => "Cuba",
				4 => "Bahamas",
				5 => "Madagascar",
				6 => "Mexico",
				7 => "India",
				8 => "Ecuador",
				9 => "Indonesia",
				10 => "California, USA",
				_ => "Unknown Location",
			}
		}

		// Parse Open-Meteo API response to check precipitation thresholds
		fn parse_open_meteo_response(
			body: &[u8],
			location_id: u8,
			insurance_types: &sp_std::collections::btree_set::BTreeSet<InsuranceType>,
		) -> Vec<(InsuranceType, u8)> {
			let mut triggered_events = Vec::new();

			let body_str = match sp_std::str::from_utf8(&body) {
				Ok(str) => str,
				Err(_) => {
					log::error!("Failed to parse response as UTF-8");
					return triggered_events;
				},
			};

			let json: Value = match serde_json::from_str(body_str) {
				Ok(json) => json,
				Err(_) => {
					log::error!("Failed to parse response as JSON");
					return triggered_events;
				},
			};

			// Extract precipitation data
			let current_precipitation = json
				.get("current")
				.and_then(|c| c.get("precipitation"))
				.and_then(|r| r.as_f64())
				.unwrap_or(0.0);

			let daily_precipitation_sum = json
				.get("daily")
				.and_then(|d| d.get("precipitation_sum"))
				.and_then(|r| r.as_array())
				.and_then(|a| a.get(0))
				.and_then(|v| v.as_f64())
				.unwrap_or(0.0);

			// The Open-Meteo API returns:
			// - "current.precipitation": current precipitation in mm for the last hour
			// - "daily.precipitation_sum": total precipitation for the day in mm
			// We should use daily_precipitation_sum as it represents the total precipitation for
			// the day
			let total_precipitation = daily_precipitation_sum;

			log::info!(
				"Location {} ({}): Current precipitation: {:.2}mm, Daily sum: {:.2}mm, Total: {:.2}mm",
				Self::get_location_name(location_id),
				location_id,
				current_precipitation,
				daily_precipitation_sum,
				total_precipitation
			);

			// Check each insurance type against its threshold
			for insurance_type in insurance_types {
				let threshold = Self::get_event_threshold(insurance_type) as f64;

				if total_precipitation >= threshold {
					log::info!(
						"Event threshold exceeded for {:?} at {} (ID: {}): {:.2}mm >= {:.2}mm",
						insurance_type,
						Self::get_location_name(location_id),
						location_id,
						total_precipitation,
						threshold
					);

					triggered_events.push((insurance_type.clone(), location_id));
				}
			}

			triggered_events
		}

		fn submit_weather_event(
			event_type: InsuranceType,
			location: u8,
		) -> Result<(), &'static str> {
			let signer = Signer::<T, <T as Config<I>>::AuthorityId>::all_accounts();
			if !signer.can_sign() {
				return Err("No accounts available for signing");
			}

			let results = signer.send_signed_transaction(|_account| {
				Call::feed_event {
					event_info: (event_type.clone(), location),
					insurances: vec![], // Empty vec means all insurances for this event type
				}
			});

			for (acc, res) in &results {
				match res {
					Ok(()) => log::info!(
						"Weather event submitted successfully. Account: {:?}, Event: {:?}, Location: {}",
						acc.id,
						event_type,
						location
					),
					Err(e) => log::error!(
						"Failed to submit weather event. Account: {:?}, Error: {:?}",
						acc.id,
						e
					),
				}
			}

			results
				.into_iter()
				.find(|(_, res)| res.is_ok())
				.map(|_| ())
				.ok_or("No transaction succeeded")
		}
	}
}
