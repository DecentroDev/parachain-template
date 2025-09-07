#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod types;
pub mod weights;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::types::{ContractLink, InsuranceMetadata};
	use frame_support::{
		pallet_prelude::*,
		traits::{
			tokens::{
				fungibles::{Create, Inspect, Mutate, Unbalanced},
				nonfungibles,
				nonfungibles::{Create as _, InspectEnumerable as _},
				Balance,
			},
			BuildGenesisConfig,
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;

	use sp_std::vec::Vec;

	use sp_runtime::traits::AtLeast32BitUnsigned;
	use crate::weights::WeightInfo;

	pub type InsuranceMetadataOf<T> = InsuranceMetadata<
		<T as Config>::Balance,
		<T as frame_system::Config>::AccountId,
		BlockNumberFor<T>,
		<T as Config>::AssetId,
		ContractLink<u8, <T as Config>::StringLimit>,
	>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_uniques::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type Balance: Balance + codec::Codec + TypeInfo + MaxEncodedLen + From<u128> + Into<u128>;

		type AssetId: AtLeast32BitUnsigned
			+ Default
			+ codec::Decode
			+ codec::EncodeLike
			+ MaxEncodedLen
			+ TypeInfo
			+ core::fmt::Debug
			+ PartialEq
			+ Clone;

		type CurrencyId: AtLeast32BitUnsigned
			+ Default
			+ codec::Decode
			+ codec::EncodeLike
			+ MaxEncodedLen
			+ TypeInfo
			+ core::fmt::Debug
			+ PartialEq
			+ Clone
			+ Into<u32>;

		type NftId: AtLeast32BitUnsigned
			+ Default
			+ codec::Decode
			+ codec::EncodeLike
			+ MaxEncodedLen
			+ TypeInfo
			+ core::fmt::Debug
			+ PartialEq
			+ Clone
			+ Into<<Self::InsuredToken as nonfungibles::Inspect<Self::AccountId>>::CollectionId>
			+ Into<<Self::InsuredToken as nonfungibles::Inspect<Self::AccountId>>::ItemId>;

		type SecondaryMarketToken: Create<Self::AccountId>
			+ Inspect<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>
			+ Mutate<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>;

		type InsuredToken: nonfungibles::Create<Self::AccountId>
			+ nonfungibles::Mutate<Self::AccountId>
			+ nonfungibles::Destroy<Self::AccountId, DestroyWitness = pallet_uniques::DestroyWitness>
			+ nonfungibles::Transfer<Self::AccountId>
			+ nonfungibles::Inspect<Self::AccountId, CollectionId = Self::NftId, ItemId = Self::NftId>
			+ nonfungibles::InspectEnumerable<Self::AccountId>;

		type StableCurrency: Inspect<Self::AccountId, AssetId = Self::CurrencyId, Balance = Self::Balance>
			+ Mutate<Self::AccountId, AssetId = Self::CurrencyId, Balance = Self::Balance>
			+ Unbalanced<Self::AccountId>;

		type UsdtId: Get<Self::CurrencyId>;

		#[pallet::constant]
		type StringLimit: Get<u32>;

		#[pallet::constant]
		type AssetMinBalance: Get<Self::Balance>;

		#[pallet::constant]
		type ZeroAddressId: Get<PalletId>;
		type WeightInfo: WeightInfo;
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config>(pub PhantomData<T>);

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self(PhantomData)
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			use sp_runtime::traits::AccountIdConversion;
			ZeroAddress::<T>::put::<T::AccountId>(
				T::ZeroAddressId::get().into_account_truncating(),
			);
		}
	}

	#[pallet::storage]
	pub type ZeroAddress<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	#[pallet::storage]
	pub type NextCollectionId<T: Config> = StorageValue<_, T::NftId, ValueQuery>;

	#[pallet::storage]
	pub type NextSmtId<T: Config> = StorageValue<_, T::AssetId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_metadata)]
	pub type Metadata<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::NftId,
		Blake2_128Concat,
		T::NftId,
		InsuranceMetadataOf<T>,
	>;

	#[pallet::storage]
	pub type SmtIdToInsurance<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AssetId, (T::NftId, T::NftId), OptionQuery>;

	#[pallet::storage]
	pub type NextItemIdForCollection<T: Config> =
		StorageMap<_, Blake2_128Concat, T::NftId, T::NftId, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Secondary market token minted
		SecondaryMarketTokensMinted {
			/// Initial mint receiver
			owner: T::AccountId,
			/// Id of the created token
			asset_id: T::AssetId,
			amount: T::Balance,
		},

		/// Insurance destroyed
		InsuranceDestroyed {
			/// Collection id
			collection_id: T::NftId,
			/// Insurance id
			insurance_id: T::NftId,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Asset ids are exhausted
		NoAvailableAssetId,

		// Can't withdraw asset, maybe it's because of not enough tokens, maybe because of
		// invalid asset id
		AssetWithdrawalFailure,

		/// Metadata is malformed
		InvalidMetadata,

		/// Non existent asset id
		AssetIdDoesNotExist,
	}

	impl<T: Config> Pallet<T> {
		pub fn get_next_collection_id() -> Result<T::NftId, DispatchError> {
			use num_traits::{bounds::Bounded, identities::One};
			<NextCollectionId<T>>::try_mutate(|n| {
				let id = n.clone();
				ensure!(id != T::NftId::max_value(), Error::<T>::NoAvailableAssetId);
				*n += T::NftId::one();
				Ok(id)
			})
		}

		fn get_next_item_id(collection_id: T::NftId) -> Result<T::NftId, DispatchError> {
			use num_traits::{bounds::Bounded, identities::One};
			<NextItemIdForCollection<T>>::try_mutate(collection_id, |n: &mut T::NftId| {
				let id = n.clone();
				ensure!(id != T::NftId::max_value(), Error::<T>::NoAvailableAssetId);
				*n += T::NftId::one();
				Ok(id)
			})
			.map_err(|_: DispatchError| Error::<T>::AssetIdDoesNotExist.into())
		}

		pub fn get_next_smt_id() -> Result<T::AssetId, DispatchError> {
			use num_traits::{bounds::Bounded, identities::One};
			<NextSmtId<T>>::try_mutate(|n| {
				let id = n.clone();
				ensure!(id != T::AssetId::max_value(), Error::<T>::NoAvailableAssetId);
				*n += T::AssetId::one();
				Ok(id)
			})
		}

		pub fn do_mint_insured_nft(
			beneficiary: T::AccountId,
			mut metadata: InsuranceMetadataOf<T>,
		) -> Result<(T::NftId, T::NftId), DispatchError> {
			use frame_support::traits::tokens::nonfungibles::Mutate;
			use num_traits::{identities::One, Zero};
			Self::validate_metadata(&metadata)?;

			metadata.status = types::InsuranceStatus::NotStarted;

			// FIXME: insurance NFTs are meant to be transferable, we should update this
			// check when implementing the transfer logic
			assert_eq!(beneficiary, metadata.creator);
			let user_items: Vec<_> = T::InsuredToken::owned(&beneficiary).collect();
			let (collection_id, item_id) = if !user_items.is_empty() {
				let collection_id = user_items[0].0.clone();
				let item_id = Self::get_next_item_id(collection_id.clone())?;

				(collection_id, item_id)
			} else {
				let collection_id = Self::get_next_collection_id()?;
				// set our zero address to be the collection admin, so that the users don't
				// burn their insurance or destroy collection by accident
				T::InsuredToken::create_collection(
					&collection_id,
					&beneficiary,
					&ZeroAddress::<T>::get()
						.expect("ZeroAddress is provided during genesis build!"),
				)?;
				NextItemIdForCollection::<T>::insert(collection_id.clone(), T::NftId::one());
				(collection_id, T::NftId::zero())
			};

			T::InsuredToken::mint_into(&collection_id, &item_id, &beneficiary)?;
			Metadata::<T>::insert(collection_id.clone(), item_id.clone(), metadata);

			Ok((collection_id, item_id))
		}

		pub fn do_mint_secondary_market_tokens(
			beneficiary: T::AccountId,
			amount: T::Balance,
		) -> Result<T::AssetId, DispatchError> {
			let smt_id = Self::get_next_smt_id()?;

			T::SecondaryMarketToken::create(
				smt_id.clone(),
				ZeroAddress::<T>::get().expect("ZeroAddress is provided during genesis build!"),
				true,
				T::AssetMinBalance::get(),
			)?;
			T::SecondaryMarketToken::mint_into(smt_id.clone(), &beneficiary, amount)?;

			Self::deposit_event(Event::SecondaryMarketTokensMinted {
				owner: beneficiary,
				asset_id: smt_id.clone(),
				amount,
			});

			Ok(smt_id)
		}

		pub fn get_user_metadata(
			collection_id: T::NftId,
			item_id: T::NftId,
		) -> Option<InsuranceMetadataOf<T>> {
			Self::get_metadata(collection_id, item_id)
		}

		pub fn get_user_assets_info(account_id: T::AccountId) -> Option<Vec<(T::NftId, T::NftId)>> {
			let items = T::InsuredToken::owned(&account_id).collect::<Vec<_>>();

			Some(items)
		}

		pub fn validate_metadata(metadata: &InsuranceMetadataOf<T>) -> DispatchResult {
			let block_number = frame_system::Pallet::<T>::block_number();
			ensure!(
				metadata.starts_on >= block_number &&
					metadata.underwrite_amount % 100_u32.into() == 0_u32.into() &&
					metadata.smt_id.is_none(),
				Error::<T>::InvalidMetadata
			);
			Ok(())
		}

		pub fn destroy_insurance(
			creator: T::AccountId,
			collection_id: T::NftId,
			insurance_id: T::NftId,
		) -> DispatchResult {
			use frame_support::traits::tokens::nonfungibles::{Destroy, Mutate};
			// TODO: insurance NFTs are meant to be transferable, we should update this
			// check when implementing the transfer logic

			T::InsuredToken::burn(&collection_id, &insurance_id, None)?;
			if T::InsuredToken::owned(&creator).next().is_none() {
				T::InsuredToken::destroy(
					collection_id.clone(),
					pallet_uniques::DestroyWitness { items: 0, item_metadatas: 0, attributes: 0 },
					Some(creator),
				)
				.unwrap();
				Self::deposit_event(crate::Event::InsuranceDestroyed {
					collection_id,
					insurance_id,
				});
			}
			Ok(())
		}
	}
}
