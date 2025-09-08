#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

pub mod traits;
pub mod types;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::{
		traits::ProvideAccountId,
		types::{GetPremiumParams, VotingMetadata},
	};
	use frame_support::{
		dispatch::PostDispatchInfo,
		pallet_prelude::*,
		traits::{
			fungibles::{Inspect, Unbalanced},
			tokens::{Fortitude, Precision, Preservation},
			Currency, ReservableCurrency, BuildGenesisConfig,
		},
		PalletId,
	};
	use frame_system::{
		ensure_root, ensure_signed,
		offchain::{AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer},
		pallet_prelude::*,
	};
	use codec::MaxEncodedLen;
	use num_traits::{CheckedDiv, CheckedSub};
	use pallet_collective::ProposalIndex;
	use pallet_insurances::pallet::InsuranceMetadataOf;
	use scale_info::prelude::{format, string::String};
	use serde_json::Value;
	use sp_core::crypto::KeyTypeId;
        use sp_runtime::{
                offchain::http,
                traits::Hash,
        };
        use sp_core::offchain::Duration;
	use sp_std::prelude::*;

	use offchain_utils::offchain_api_key::OffchainApiKey;

	pub struct CustomApiKeyFetcher;

	impl OffchainApiKey for CustomApiKeyFetcher {}

	pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"ofwr");
	// Temporary value for requesting premium price
	pub const GET_RAIN_FALL_API_REQUEST: &str = "http://127.0.0.1:9090/pricing?";

	/// Based on the above `KeyTypeId` we need to generate a pallet-specific crypto type wrappers.
	/// We can use from supported crypto kinds (`sr25519`, `ed25519` and `ecdsa`) and augment
	/// the types with this pallet-specific identifier.
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

		// implemented for mock runtime in test
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
	#[pallet::without_storage_info]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::config]
	pub trait Config<I: 'static = ()>:
		frame_system::Config
		+ pallet_insurances::Config
		+ pallet_collective::Config<I>
		+ pallet_assets::Config
		+ CreateSignedTransaction<Call<Self, I>>
	where
		<Self as frame_system::Config>::AccountId: MaxEncodedLen,
	{
		/// The identifier type for an offchain worker.
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;

		/// The overarching dispatch call type.
		type Call: From<Call<Self, I>>;

		type DaoOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

		type RuntimeEvent: From<Event<Self, I>>
			+ IsType<<Self as frame_system::Config>::RuntimeEvent>
			+ TryInto<Event<Self, I>>;

		type LocalCurrency: ReservableCurrency<<Self as frame_system::Config>::AccountId>
			+ Currency<
				<Self as frame_system::Config>::AccountId,
				Balance = <Self as pallet_insurances::Config>::Balance,
			>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		#[pallet::constant]
		type Quorum: Get<u32>;

		#[pallet::constant]
		type MaxProposalWeight: Get<u64>;

		#[pallet::constant]
		type MaxLengthBound: Get<u32>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config<I>, I: 'static = ()>(pub PhantomData<(T, I)>);

	impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
		fn default() -> Self {
			Self(PhantomData)
		}
	}

	#[pallet::genesis_build]
	impl<T: Config<I>, I: 'static> BuildGenesisConfig for GenesisConfig<T, I> {
		fn build(&self) {
			use sp_runtime::traits::AccountIdConversion;
			PalletAccountId::<T, I>::put::<T::AccountId>(
				T::PalletId::get().into_account_truncating(),
			);
		}
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I>
	where
		<T as frame_system::Config>::RuntimeEvent: From<pallet::Event<T, I>>,
		<T as frame_system::Config>::RuntimeEvent: TryInto<pallet::Event<T, I>>,
	{
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			<T as Config<I>>::WeightInfo::do_execute_voting_ended()
		}

		fn on_finalize(n: BlockNumberFor<T>) {
			Self::do_execute_voting_ended(n)
		}

		fn offchain_worker(_n: BlockNumberFor<T>) {
			log::info!("Ping from offchain workers! : {}", GET_RAIN_FALL_API_REQUEST);
			let signer = Signer::<T, T::AuthorityId>::all_accounts();

			if !signer.can_sign() {
				log::error!("No available keys for signing in the keystore!");
				return;
			}

			
			for event in frame_system::Pallet::<T>::read_events_no_consensus() {
				
				if let Ok(Event::<T, I>::GetInsurancePremium {
					account_id,
					get_premium_params,
					mut metadata,
					proposal,
				}) = event.event.try_into()
				{
					log::info!("GetInsurancePremium event: {:?}", account_id);
					match Pallet::<T, I>::fetch_premium_amount(
						account_id.clone(),
						get_premium_params.clone(),
					) {
						Ok(price) => {
							metadata.premium_amount = price;
							let metadata = metadata.clone();
							let proposal = proposal.clone();

							log::warn!("Metadata: {:?}", metadata);

						

							// Using `send_signed_transaction` associated type we create and submit
							// a transaction representing the call we've just created.
							// `send_signed_transaction()` return type is `Option<(Account<T>,
							// Result<(), ()>)>`. It is:
							// 	 - `None`: no account is available for sending transaction
							// 	 - `Some((account, Ok(())))`: transaction is successfully sent
							// 	 - `Some((account, Err(())))`: error occurred when sending the
							//     transaction
							let results = signer.send_signed_transaction(|_account| {
								Call::request_insurance {
									metadata: metadata.clone(),
									proposal: proposal.clone(),
								}
							});

							if !results.is_empty() {
								for (acc, res) in &results {
									match res {
										Ok(()) => log::warn!(
											"[{:?}]: submit transaction success.",
											acc.id
										),
										Err(e) => log::error!(
											"[{:?}]: submit transaction failure. Reason: {:?}",
											acc.id,
											e
										),
									}
								}
							} else {
								log::error!("Failed to send signed transaction");
							}
						},
						Err(_) => Self::deposit_event(Event::<T, I>::FailedToGetPremium {
							account_id,
							get_premium_params,
							metadata,
							proposal,
						}),
					};
				} else {
					log::debug!("😈 No insurances were requested!")
				}
			}
		}
	}

	#[pallet::storage]
	pub type NextProposalIndex<T: Config<I>, I: 'static = ()> =
		StorageValue<_, ProposalIndex, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pallet_account_id)]
	pub type PalletAccountId<T: Config<I>, I: 'static = ()> = StorageValue<_, T::AccountId>;

	pub type VotingMetadataOf<T> = VotingMetadata<
		BlockNumberFor<T>,
		<T as frame_system::Config>::AccountId,
		<T as pallet_insurances::Config>::Balance,
		<T as frame_system::Config>::Hash,
		ProposalIndex,
	>;

	#[pallet::storage]
	#[pallet::getter(fn active_voting_metadata)]
	pub type ActiveVotingMetadata<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Identity, T::Hash, VotingMetadataOf<T>, OptionQuery>;

	/// Used for voting finalization.
	///
	/// When finalizing the voting, we invoke [`do_close`] from pallet
	/// [`Collective`](pallet_collective::Pallet). It does provide info about if the proposal was
	/// approved/disapproved, but does not indicate in any way if the proposal execution was
	/// successful. This value allows us to know if the proposal was executed successfully and if we
	/// should unreserve user's funds.
	///
	/// The usage of this value is as follows:
	/// 1. Set this value to `true`.
	/// 2. Invoke the [`do_close`].
	/// 3. In the proposal handler, on successful execution, set this value to `false`.
	/// 4. After the proposal is closed, check this value and unreserve user's premium if needed.
	///
	/// [`do_close`]: pallet_collective::Pallet::do_close
	#[pallet::storage]
	pub type ShouldUnreserveFunds<T: Config<I>, I: 'static = ()> =
		StorageValue<_, bool, ValueQuery>;

	/// Contains auxiliary data to be accessed from inside of a proposal handler.
	///
	/// In case of proposal approval, the request identifier will be stored in
	/// `pallet_insurances::InsuranceRequestIdentifiers` and the proposal index will be emitted in
	/// the event.
	///
	/// The data contained in this map is ethereal and must exist only while the proposal is being
	/// voted on.
	///
	/// The hash in the map key is the hash of metadata which is being proposed to DAO.
	#[pallet::storage]
	#[pallet::getter(fn pending_proposal_info)]
	pub type PendingProposalInfo<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Identity, (T::AccountId, T::Hash), ProposalIndex, OptionQuery>;

	#[pallet::storage]
	pub type DaoMembersForInsurance<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Identity, (T::NftId, T::NftId), Vec<T::AccountId>, OptionQuery>;

	#[pallet::storage]
	pub type ParamsForPremiumCalculation<T: Config<I>, I: 'static = ()> = StorageValue<
		_,
		Vec<(T::AccountId, GetPremiumParams<<T as pallet_insurances::Config>::Balance>, bool)>,
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// New DAO member added
		DaoMemberAdded {
			/// Account id of the member
			new_member: T::AccountId,
		},

		/// New DAO member added
		DaoMemberRemoved {
			/// Account id of the member
			old_member: T::AccountId,
		},

		/// Voted on the insurance request
		LiquidityProvisionVoted {
			/// Voter
			who: T::AccountId,
			/// Request's proposal index
			proposal_index: ProposalIndex,
			/// Member decision
			decision: bool,
		},

		/// Liquidity allocated to user
		LiquidityAllocated {
			/// Request's proposal index
			proposal_index: ProposalIndex,
			/// Insurance receiver
			who: T::AccountId,
			/// Insurance collection id
			collection_id: T::NftId,
			/// Insurance id
			item_id: T::NftId,
		},

		/// Insurance request created
		InsuranceRequested {
			/// Metadata of the requested insurance
			metadata: InsuranceMetadataOf<T>,
			/// Hash of the proposal
			proposal_hash: T::Hash,
			/// Index of the proposal
			proposal_index: ProposalIndex,
		},

		/// The insurance request was disapproved.
		/// This is a separate event since we must combine
		/// the proposal hash and the proposal index to
		/// uniquely identify the insurance request.
		InsuranceRequestDisapproved {
			/// Hash of the proposal
			proposal_hash: T::Hash,
			/// Index of the proposal
			proposal_index: ProposalIndex,
		},

		/// Dao claimed profits
		DaoClaimedProfits {
			/// Collection id
			collection_id: T::NftId,
			/// Item id
			item_id: T::NftId,
		},

		/// Get premium amount for insurance
		GetInsurancePremium {
			/// Account id for insurance
			account_id: T::AccountId,
			/// Params to get premium amount
			get_premium_params: GetPremiumParams<<T as pallet_insurances::Config>::Balance>,
			/// Requested insurance metadata
			metadata: InsuranceMetadataOf<T>,
			/// Requested insurance proposal
			proposal: Box<<T as pallet_collective::Config<I>>::Proposal>,
		},

		/// Failed to get premium amount
		FailedToGetPremium {
			/// Account id for insurance
			account_id: T::AccountId,
			/// Params to get premium amount
			get_premium_params: GetPremiumParams<<T as pallet_insurances::Config>::Balance>,
			/// Requested insurance metadata
			metadata: InsuranceMetadataOf<T>,
			/// Requested insurance proposal
			proposal: Box<<T as pallet_collective::Config<I>>::Proposal>,
		},
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Account is not a DAO member
		NotDaoMember,

		/// The account doesn't exist in `Authorities`
		NotAuthority,

		/// The account already exists in `Authorities`
		AlreadyAuthority,

		/// Account already in DAO members list
		AlreadyDaoMember,

		/// Proposal indexes are exhausted
		NoAvailableProposalIndex,

		/// Account does not have enough funds
		NotEnoughFunds,

		/// Could not find proposal index
		ProposalIndexNotFound,

		/// Unknown insurance
		NoMetadataFound,

		/// No DAO members are eligible for this insurance premium
		NoDaoMembersForInsurance,

		/// DAO is not eligible for premium, as the liquidity was bought out
		DaoNotEligibleForPremium,

		/// Duplicate user/insurance metadata pair
		DuplicateProposal,

		/// Failed to parse price from response
		FailedToParsePremium,
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config<I>>::WeightInfo::add_member())]
		#[frame_support::transactional]
		pub fn add_member(origin: OriginFor<T>, new_member: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(Self::is_dao_member(&new_member).is_err(), Error::<T, I>::AlreadyDaoMember);

			pallet_collective::Members::<T, I>::append(new_member.clone());

			Self::deposit_event(Event::<T, I>::DaoMemberAdded { new_member });

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config<I>>::WeightInfo::remove_member())]
		#[frame_support::transactional]
		pub fn remove_member(origin: OriginFor<T>, old_member: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;

			Self::is_dao_member(&old_member)?;

			pallet_collective::Members::<T, I>::mutate(|members| {
				members.retain(|account_id| *account_id != old_member);
			});

			Self::deposit_event(Event::DaoMemberRemoved { old_member });

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config<I>>::WeightInfo::request_insurance())]
		#[frame_support::transactional]
		pub fn request_insurance(
			origin: OriginFor<T>,
			mut metadata: InsuranceMetadataOf<T>,
			proposal: Box<<T as pallet_collective::Config<I>>::Proposal>,
		) -> DispatchResult {
			ensure_signed(origin)?;
			let usdt_id = <T as pallet_insurances::Config>::UsdtId::get();

			pallet_insurances::Pallet::<T>::validate_metadata(&metadata)?;

			let proposal_index = Self::get_next_proposal_index()?;

			let _new_balance = T::StableCurrency::reducible_balance(
				usdt_id.clone(),
				&metadata.creator,
				Preservation::Preserve,
				Fortitude::Polite,
			)
			.checked_sub(&metadata.premium_amount)
			.ok_or(Error::<T, I>::NotEnoughFunds)?;

			// check for balance is available to lock or not
			T::StableCurrency::can_withdraw(
				usdt_id.clone(),
				&metadata.creator,
				metadata.premium_amount,
			)
			.into_result(true)
			.map_err(|_| Error::<T, I>::NotEnoughFunds)?;


			let pallet_account_id = Self::account_id();
			T::StableCurrency::can_withdraw(
				usdt_id,
				&pallet_account_id,
				metadata.underwrite_amount,
			)
			.into_result(true)
			.map_err(|_| Error::<T, I>::NotEnoughFunds)?;

			T::StableCurrency::decrease_balance(
				T::UsdtId::get(),
				&metadata.creator,
				metadata.premium_amount,
				Precision::Exact,
				Preservation::Preserve,
				Fortitude::Polite,
			)?;

			let (_proposal_len, _active_proposals) =
				pallet_collective::Pallet::<T, I>::do_propose_proposed(
					metadata.creator.clone(),
					T::Quorum::get(),
					proposal.clone(),
					<T as pallet::Config<I>>::MaxProposalWeight::get() as u32,
				)?;

			let proposal_hash = T::Hashing::hash_of(&proposal);

			let end = frame_system::Pallet::<T>::block_number() + T::MotionDuration::get();
			// To get rid of errors with `metadata_hash` when we'll get original `premium_amount`
			let premium_amount = metadata.premium_amount;
			metadata.premium_amount = 0_u32.into();
			let metadata_hash = T::Hashing::hash_of(&metadata);
			metadata.premium_amount = premium_amount;
			ActiveVotingMetadata::<T, I>::insert(
				proposal_hash,
				VotingMetadata {
					ends_on: end,
					beneficiary: metadata.creator.clone(),
					premium_amount: metadata.premium_amount,
					metadata_hash,
					proposal_index,
				},
			);

			ensure!(
				PendingProposalInfo::<T, I>::get((metadata.creator.clone(), metadata_hash))
					.is_none(),
				Error::<T, I>::DuplicateProposal
			);
			PendingProposalInfo::<T, I>::insert(
				(metadata.creator.clone(), metadata_hash),
				proposal_index,
			);

			Self::deposit_event(Event::InsuranceRequested {
				metadata,
				proposal_hash,
				proposal_index,
			});

			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config<I>>::WeightInfo::vote())]
		#[frame_support::transactional]
		pub fn vote(
			origin: OriginFor<T>,
			proposal_hash: T::Hash,
			index: ProposalIndex,
			approve: bool,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::is_dao_member(&who)?;

			pallet_collective::Pallet::<T, I>::do_vote(who.clone(), proposal_hash, index, approve)?;

			Self::deposit_event(Event::LiquidityProvisionVoted {
				who,
				proposal_index: index,
				decision: approve,
			});

			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config<I>>::WeightInfo::allocate_liquidity())]
		#[frame_support::transactional]
		pub fn allocate_liquidity(
			origin: OriginFor<T>,
			metadata: InsuranceMetadataOf<T>,
		) -> DispatchResult {
			let _ = <T as Config<I>>::DaoOrigin::ensure_origin(origin)?;

			Self::do_allocate_liquidity(metadata)?;

			// Proposal executed successfully, do not return premium to the user
			ShouldUnreserveFunds::<T, I>::set(false);

			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(<T as Config<I>>::WeightInfo::allocate_liquidity())]
		#[frame_support::transactional]
		pub fn prepare_request_insurance(
			origin: OriginFor<T>,
			params: GetPremiumParams<<T as pallet_insurances::Config>::Balance>,
			metadata: InsuranceMetadataOf<T>,
			proposal: Box<<T as pallet_collective::Config<I>>::Proposal>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			if let Some(mut data) = ParamsForPremiumCalculation::<T, I>::get() {
				data.push((who.clone(), params.clone(), false))
			}

			Self::deposit_event(Event::GetInsurancePremium {
				account_id: who,
				get_premium_params: params,
				metadata,
				proposal,
			});

			Ok(())
		}
	}

	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		fn fetch_premium_amount(
			_account_id: T::AccountId,
			params: GetPremiumParams<<T as pallet_insurances::Config>::Balance>,
		) -> Result<<T as pallet_insurances::Config>::Balance, http::Error> {
			// We want to keep the offchain worker execution time reasonable, so we set a hard-coded
			// deadline to 2s to complete the external call.
			let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(12_000));

			let latitude = Self::floating_string(params.latitude);
			let longtitude = Self::floating_string(params.longtitude);
			let start_date = params.start_date.into();
			let duration_in_hours = params.duration_in_hours.into();
			let threshold = params.threshold.into();
			let coverage = params.coverage.into();
			let number_of_simulations = params.number_of_simulations.into();
			let roc = Self::floating_string(params.roc);

			// Retrieve the API key from offchain storage
			let api_key = match CustomApiKeyFetcher::fetch_api_key_for_request("pricing_api_key") {
				Ok(key) => key,
				Err(err) => {
					log::error!("Failed to fetch API key: {}", err);
					return Err(http::Error::Unknown);
				},
			};


			// Initiate an external HTTP GET request.
			let api_request = format!(
				"{}lat={}&lon={}&startdate={}&duration_in_hours={}&threshold={}&coverage={}&number_of_simulations={}&ROC={}",
				GET_RAIN_FALL_API_REQUEST, latitude, longtitude, start_date, duration_in_hours,
				threshold, coverage, number_of_simulations, roc
			);
			let request = http::Request::get(&api_request).add_header("X_API_KEY", &api_key);

			// We set the deadline for sending of the request, note that awaiting response can
			// have a separate deadline. Next we send the request.
			let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;

			// The request is already being processed by the host, we are free to do anything
			// else in the worker (we can send multiple concurrent requests too).
			let response =
				pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;

			// Let's check the status code before we proceed to reading the response.
			if response.code != 200 {
				log::warn!("Unexpected status code: {}", response.code);
				return Err(http::Error::Unknown);
			}

			let body = response.body().collect::<Vec<u8>>();

			log::info!("Response: {:?}", body);

			let price = match Self::parse_premium(&body) {
				Some(price) => Ok(price),
				None => {
					log::warn!("Unable to extract price from the response: {:?}", body);
					Err(http::Error::Unknown)
				},
			}?;

			log::info!("Premium amount {}", &price);

			Ok(price.into())
		}

		fn floating_string(value: <T as pallet_insurances::Config>::Balance) -> String {
			let mut value_str = format!("{}", value.into());
			while value_str.len() < 13 {
				value_str.insert(0, '0')
			}
			value_str.insert(value_str.len() - 12, '.');
			value_str
		}

		/// Parse the premium_amount from the given JSON string using `lite-json`.
		///
		/// Returns `None` when parsing failed or `Some(price in dollars * 10^12)` when parsing is
		/// successful.
		fn parse_premium(price_str: &[u8]) -> Option<u128> {
			let value: serde_json::Value = serde_json::from_slice(price_str)
				.or(Err(Error::<T, I>::FailedToParsePremium))
				.ok()?;

			let result = Self::match_json_value(value);

			Some(result.unwrap())
		}

		pub fn match_json_value(value: serde_json::Value) -> Result<u128, DispatchError> {
			match value {
				Value::Object(obj) => {
					let (_, v) = obj
						.into_iter()
						.find(|(k, _)| k.chars().eq("recommended_premium".chars()))
						.unwrap();
					let result = match v {
						Value::Number(number) => {
							let number = number.as_str();
							u128::from_str_radix(&number, 10)
								.map_err(|_| "Failed to parse from string".into())
						},
						_ => {
							log::warn!("Fail to match v");
							Err("Not a number".into())
						},
					};
					result
				},
				Value::Array(array) => {
					let mut result = Err("Failed to get premium from array".into());
					for value in array {
						result = Self::match_json_value(value);
					}
					result
				},
				Value::String(string) => {
					let cleaned_str =
						string.replace(&['{', '[', '}', ']', '\"', '.', ';', '\''][..], "");

					let splited = cleaned_str.split(",");
					let mut last_gen = vec![];
					let mut result = Err("Incorrect json".into());
					for str in splited {
						last_gen.push(str.split(":"));
						if str.contains("recommended_premium:") {
							let rep_str = str.replace("recommended_premium:", "");
							result = u128::from_str_radix(&rep_str, 10)
								.map_err(|_| "Failed to parse from string".into())
						}
					}
					result
				},
				Value::Number(number) => {
					let number = number.as_str();
					return u128::from_str_radix(number, 10)
						.map_err(|_| "Failed to parse from number".into())
				},
				Value::Null => return Err("Null value".into()),
				_ => return Err("Value isn't ok".into()),
			}
		}

		fn get_next_proposal_index() -> Result<ProposalIndex, DispatchError> {
			NextProposalIndex::<T, I>::try_mutate(|n| {
				let id = *n;
				ensure!(id != ProposalIndex::max_value(), Error::<T, I>::NoAvailableProposalIndex);
				*n += 1;
				Ok(id)
			})
		}

		fn do_allocate_liquidity(mut metadata: InsuranceMetadataOf<T>) -> DispatchResult {
			let dao_account_id = Self::account_id();
			let usdt_id = <T as pallet_insurances::Config>::UsdtId::get();

			// To get rid of errors with `metadata_hash`
			// that we had before original `premium_amount`
			let premium_amount = metadata.premium_amount;
			metadata.premium_amount = 0_u32.into();

			let metadata_hash = T::Hashing::hash_of(&metadata);
			metadata.premium_amount = premium_amount;
			let proposal_index =
				PendingProposalInfo::<T, I>::take((metadata.creator.clone(), metadata_hash))
					.ok_or(Error::<T, I>::ProposalIndexNotFound)?;

			// Lock funds for this insurance
			let _new_balance = T::StableCurrency::reducible_balance(
				usdt_id.clone(),
				&dao_account_id,
				Preservation::Preserve,
				Fortitude::Polite,
			)
			.checked_sub(&metadata.underwrite_amount)
			.ok_or(Error::<T, I>::NotEnoughFunds)?;

			T::StableCurrency::can_withdraw(
				usdt_id.clone(),
				&dao_account_id,
				metadata.underwrite_amount,
			)
			.into_result(true)
			.map_err(|_| Error::<T, I>::NotEnoughFunds)?;

			T::StableCurrency::decrease_balance(
				usdt_id,
				&dao_account_id,
				metadata.underwrite_amount,
				Precision::Exact,
				Preservation::Preserve,
				Fortitude::Polite,
			)?;

			// Call nft mint
			let (collection_id, asset_id) = pallet_insurances::Pallet::<T>::do_mint_insured_nft(
				metadata.creator.clone(),
				metadata.clone(),
			)?;

			let members = pallet_collective::Members::<T, I>::get();

			DaoMembersForInsurance::<T, I>::insert(
				(collection_id.clone(), asset_id.clone()),
				members,
			);

			Self::deposit_event(Event::LiquidityAllocated {
				proposal_index,
				who: metadata.creator,
				collection_id,
				item_id: asset_id,
			});

			Ok(())
		}

		pub fn do_claim_dao_profits(collection_id: T::NftId, item_id: T::NftId) -> DispatchResult {
			let dao_account = Self::account_id();
			let metadata =
				pallet_insurances::Metadata::<T>::get(collection_id.clone(), item_id.clone())
					.ok_or(Error::<T, I>::NoMetadataFound)?;

			ensure!(metadata.smt_id.is_none(), Error::<T, I>::DaoNotEligibleForPremium);

			let members =
				DaoMembersForInsurance::<T, I>::get((collection_id.clone(), item_id.clone()))
					.ok_or(Error::<T, I>::NoDaoMembersForInsurance)?;

			let members_count = members.len() as u32;

			let _new_balance = T::StableCurrency::reducible_balance(
				T::UsdtId::get(),
				&dao_account,
				Preservation::Preserve,
				Fortitude::Polite,
			)
			.checked_sub(&metadata.premium_amount)
			.ok_or(Error::<T, I>::NotEnoughFunds)?;

			T::StableCurrency::can_withdraw(
				T::UsdtId::get(),
				&dao_account,
				metadata.premium_amount,
			)
			.into_result(true)
			.map_err(|_| Error::<T, I>::NotEnoughFunds)?;

			let dao_profit = metadata
				.premium_amount
				.checked_div(&members_count.into())
				.expect("Dao members must not be empty");
			for member in members {
				T::StableCurrency::decrease_balance(
					T::UsdtId::get(),
					&dao_account,
					dao_profit,
					Precision::Exact,
					Preservation::Preserve,
					Fortitude::Polite,
				)?;
				T::StableCurrency::increase_balance(
					T::UsdtId::get(),
					&member,
					dao_profit,
					Precision::Exact,
				)?;
			}

			Self::deposit_event(Event::DaoClaimedProfits { collection_id, item_id });

			Ok(())
		}

		pub fn do_execute_voting_ended(now: BlockNumberFor<T>) {
			ActiveVotingMetadata::<T, I>::translate(
				|proposal_hash, voting_metadata: VotingMetadataOf<T>| {
					if voting_metadata.ends_on <= now {
						ShouldUnreserveFunds::<T, I>::set(true);
						let result = pallet_collective::Pallet::<T, I>::do_close(
							proposal_hash,
							voting_metadata.proposal_index,
							<T as pallet_collective::Config<I>>::MaxProposalWeight::get(),
							T::MaxLengthBound::get(),
						);

						if let Err(e) = result {
							log::error!(target: "runtime::dao", "Error while closing proposal {:?}: {:?}", proposal_hash, e);
							return Some(voting_metadata);
						}

						if
						// Proposal was disapproved
						matches!(result, Ok(PostDispatchInfo { pays_fee: Pays::No, .. })) {
							T::StableCurrency::increase_balance(
								T::UsdtId::get(),
								&voting_metadata.beneficiary,
								voting_metadata.premium_amount,
								Precision::Exact,
							)
							.ok()?;
							Self::deposit_event(Event::<T, I>::InsuranceRequestDisapproved {
								proposal_hash,
								proposal_index: voting_metadata.proposal_index,
							});
						}
						// Proposal execution failed
						else if ShouldUnreserveFunds::<T, I>::take() {
							T::StableCurrency::increase_balance(
								T::UsdtId::get(),
								&voting_metadata.beneficiary,
								voting_metadata.premium_amount,
								Precision::Exact,
							)
							.ok()?;
						}

						PendingProposalInfo::<T, I>::remove((
							voting_metadata.beneficiary,
							voting_metadata.metadata_hash,
						));
						None
					} else {
						Some(voting_metadata)
					}
				},
			);
		}

		pub fn is_dao_member(account_id: &T::AccountId) -> DispatchResult {
			if !pallet_collective::Members::<T, I>::get().contains(account_id) {
				return Err(Error::<T, I>::NotDaoMember.into());
			}
			Ok(())
		}
	}

	impl<T: Config<I>, I: 'static> ProvideAccountId<T::AccountId> for Pallet<T, I> {
		fn account_id() -> T::AccountId {
			PalletAccountId::<T, I>::get()
				.expect("Dao Account Id is always provided by the genesis build")
		}
	}
}
