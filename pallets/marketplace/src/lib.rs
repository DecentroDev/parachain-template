#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod types;

pub mod weights;
pub use weights::*;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungibles::{Inspect, Unbalanced},
			tokens::{fungibles::Mutate, Fortitude, Precision, Preservation, WithdrawConsequence},
			Currency, ReservableCurrency,
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use num_traits::{CheckedDiv, Zero};
	use sp_runtime::traits::AtLeast32BitUnsigned;
	use sp_std::{cmp::Ordering, prelude::Vec};

	use pallet_dao::traits::ProvideAccountId as ProvideDaoAccountId;

	use types::{OrderInfo, OrderType};
	use weights::WeightInfo;

	#[pallet::pallet]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::config]
	pub trait Config<I: 'static = ()>:
		frame_system::Config
		+ pallet_insurances::Config
		+ pallet_assets::Config
		+ pallet_dao::Config<I>
	{
		type RuntimeEvent: From<Event<Self, I>>
			+ IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type OrderId: Default
			+ core::fmt::Debug
			+ AtLeast32BitUnsigned
			+ codec::Decode
			+ codec::EncodeLike
			+ MaxEncodedLen
			+ TypeInfo;

		type WeightInfo: crate::WeightInfo;

		type Currency: Currency<
				<Self as frame_system::Config>::AccountId,
				Balance = <Self as pallet_insurances::Config>::Balance,
			> + ReservableCurrency<<Self as frame_system::Config>::AccountId>;

		type DaoAccountIdProvider: ProvideDaoAccountId<Self::AccountId>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		#[pallet::constant]
		type BaseSecondaryMarketTokenPrice: Get<<Self as pallet_insurances::Config>::Balance>;

		#[pallet::constant]
		type MaxFulfillers: Get<u32>;
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config<I>, I: 'static = ()>(pub PhantomData<(T, I)>);

	#[cfg(feature = "std")]
	impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
		fn default() -> Self {
			Self(PhantomData)
		}
	}

	#[pallet::genesis_build]
	impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig<T, I> {
		fn build(&self) {
			use sp_runtime::traits::AccountIdConversion;
			PalletAccountId::<T, I>::put::<T::AccountId>(
				<T as pallet::Config<I>>::PalletId::get().into_account_truncating(),
			);
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn pallet_account_id)]
	pub type PalletAccountId<T: Config<I>, I: 'static = ()> = StorageValue<_, T::AccountId>;

	#[pallet::storage]
	pub type NextOrderId<T: Config<I>, I: 'static = ()> = StorageValue<_, T::OrderId, ValueQuery>;

	pub type OrderInfoOf<T> = OrderInfo<
		<T as frame_system::Config>::AccountId,
		<<T as pallet_insurances::Config>::SecondaryMarketToken as frame_support::traits::fungibles::Inspect<<T as frame_system::Config>::AccountId>>::AssetId,
		<T as pallet_insurances::Config>::Balance,
	>;

	#[pallet::storage]
	pub type OrderBook<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, T::OrderId, OrderInfoOf<T>>;

	#[pallet::storage]
	pub type OrderSMTsAmount<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		Blake2_128Concat,
		T::OrderId,
		BoundedVec<(T::AccountId, <T as pallet_insurances::Config>::Balance), T::MaxFulfillers>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// Insurance liquidity bought out from DAO
		LiquidityProvided {
			/// New insurance liquidity provider
			who: T::AccountId,
			/// Secondary market token id for this insurance
			smt_id: <T as pallet_insurances::Config>::AssetId,
			/// Insurance collection id
			collection_id: T::NftId,
			/// Insurance id
			item_id: T::NftId,
		},

		/// Order created
		OrderCreated {
			/// Order id
			id: T::OrderId,
			/// Order info
			info: OrderInfoOf<T>,
		},

		/// Order fulfilled
		OrderFulfilled {
			/// Order id
			id: T::OrderId,
			who: T::AccountId,
		},

		/// Order partially fulfilled
		OrderPartiallyFulfilled {
			/// Order id
			id: T::OrderId,
			who: T::AccountId,
			amount: <T as pallet_insurances::Config>::Balance,
			residual: <T as pallet_insurances::Config>::Balance,
		},

		/// Order canceled
		OrderCanceled {
			/// Order id
			id: T::OrderId,
		},
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Unknown insurance
		NoMetadataFound,

		/// If Insurance with specific id was already sold
		InsuranceAlreadySold,

		/// Account does not have enough funds
		NotEnoughFunds,

		/// Order ids are exhausted
		NoAvailableOrderId,

		/// Order with this id does not exist
		InvalidOrderId,

		/// Order creator and caller do not match
		OrderCreatorMismatch,

		/// Could not unreserve currency
		UnreserveFailure,

		/// The base price for an SMT was set to zero
		BaseSecondaryMarketTokenPriceIsZero,

		/// Got invalid amount of tokens
		InvalidTokensAmount,

		/// Exceeds limit of max fulfillers count
		ExceedsMaxFulfillersCount,
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config<I>>::WeightInfo::provide_liquidity())]
		#[frame_support::transactional]
		pub fn provide_liquidity(
			origin: OriginFor<T>,
			collection_id: T::NftId,
			item_id: T::NftId,
		) -> DispatchResult {
			use pallet_insurances::Metadata;

			let dao_account_id = T::DaoAccountIdProvider::account_id();
			let caller = ensure_signed(origin)?;

			let details = Metadata::<T>::get(collection_id.clone(), item_id.clone())
				.ok_or(Error::<T, I>::NoMetadataFound)?;
			ensure!(details.smt_id.is_none(), Error::<T, I>::InsuranceAlreadySold);

			let underwrite_amount = details.underwrite_amount;
			let tokens = underwrite_amount
				.checked_div(&T::BaseSecondaryMarketTokenPrice::get())
				.ok_or(Error::<T, I>::BaseSecondaryMarketTokenPriceIsZero)?;

			<T as pallet_insurances::Config>::StableCurrency::can_withdraw(
				<T as pallet_insurances::Config>::UsdtId::get(),
				&caller,
				underwrite_amount,
			)
			.into_result(true)
			.map_err(|_| Error::<T, I>::NotEnoughFunds)?;

			<T as pallet_insurances::Config>::StableCurrency::transfer(
				<T as pallet_insurances::Config>::UsdtId::get(),
				&caller,
				&dao_account_id,
				underwrite_amount,
				Preservation::Preserve,
			)?;

			// Mint SecondaryMarketTokens for insurance_id
			let smt_id = pallet_insurances::Pallet::<T>::do_mint_secondary_market_tokens(
				caller.clone(),
				tokens,
			)?;

			Metadata::<T>::mutate(collection_id.clone(), item_id.clone(), |metadata| {
				if let Some(metadata) = metadata {
					metadata.smt_id = Some(smt_id.clone())
				};
			});
			pallet_insurances::SmtIdToInsurance::<T>::insert(
				smt_id.clone(),
				(collection_id.clone(), item_id.clone()),
			);

			Self::deposit_event(Event::LiquidityProvided {
				who: caller,
				smt_id,
				collection_id,
				item_id,
			});

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight((<T as Config<I>>::WeightInfo::create_order_sell() + <T as Config<I>>::WeightInfo::create_order_buy()) / 2)]
		#[frame_support::transactional]
		pub fn create_order(
			origin: OriginFor<T>,
			token_id: <T as pallet_insurances::Config>::AssetId,
			token_amount: <T as pallet_insurances::Config>::Balance,
			price_per_token: <T as pallet_insurances::Config>::Balance,
			order_type: OrderType,
		) -> DispatchResult {
			let creator = ensure_signed(origin)?;
			let order_id = Self::get_next_order_id()?;

			match order_type {
				OrderType::Buy => {
					let amount_to_withdraw = token_amount * price_per_token;
					<T as pallet_insurances::Config>::StableCurrency::decrease_balance(
						<T as pallet_insurances::Config>::UsdtId::get(),
						&creator,
						amount_to_withdraw,
						Precision::Exact,
						Preservation::Preserve,
						Fortitude::Polite,
					)
					.or(Err(Error::<T, I>::NotEnoughFunds))?;
				},
				OrderType::Sell => {
					T::SecondaryMarketToken::transfer(
						token_id.clone(),
						&creator,
						&PalletAccountId::<T, I>::get().unwrap(),
						token_amount,
						Preservation::Expendable,
					)
					.or(Err(Error::<T, I>::InvalidTokensAmount))?;
				},
			}

			let order_info =
				OrderInfoOf::<T> { creator, order_type, token_id, token_amount, price_per_token };
			OrderBook::<T, I>::insert(order_id.clone(), order_info.clone());
			Self::deposit_event(Event::OrderCreated { id: order_id, info: order_info });

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight((<T as Config<I>>::WeightInfo::cancel_order_sell() + <T as Config<I>>::WeightInfo::cancel_order_buy()) / 2)]
		#[frame_support::transactional]
		pub fn cancel_order(origin: OriginFor<T>, order_id: T::OrderId) -> DispatchResult {
			let creator = ensure_signed(origin)?;
			let order_info =
				OrderBook::<T, I>::get(order_id.clone()).ok_or(Error::<T, I>::InvalidOrderId)?;

			if order_info.creator != creator {
				return Err(Error::<T, I>::OrderCreatorMismatch.into());
			}

			Self::do_cancel_order(creator, order_id.clone())?;

			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight((<T as Config<I>>::WeightInfo::fulfill_order_sell() + <T as Config<I>>::WeightInfo::fulfill_order_buy()) / 2)]
		#[frame_support::transactional]
		pub fn fulfill_order(
			origin: OriginFor<T>,
			order_id: T::OrderId,
			amount: <T as pallet_insurances::Config>::Balance,
		) -> DispatchResult {
			ensure!(
				amount != <T as pallet_insurances::Config>::Balance::zero(),
				Error::<T, I>::InvalidTokensAmount
			);
			let fulfiller = ensure_signed(origin)?;
			let order_info =
				OrderBook::<T, I>::get(order_id.clone()).ok_or(Error::<T, I>::InvalidOrderId)?;
			let mut available_tokens_amount = order_info.token_amount;
			let reserved_tokens_amount = OrderSMTsAmount::<T, I>::get(order_id.clone())
				.into_iter()
				.fold(<T as pallet_insurances::Config>::Balance::zero(), |acc, (_, amount)| {
					acc + amount
				});

			available_tokens_amount -= reserved_tokens_amount;
			match amount.cmp(&available_tokens_amount) {
				Ordering::Equal => {
					OrderSMTsAmount::<T, I>::try_mutate(order_id.clone(), |vec_data| {
						vec_data
							.try_push((fulfiller.clone(), amount))
							.map_err(|_| Error::<T, I>::ExceedsMaxFulfillersCount)
					})?;
					match order_info.order_type {
						OrderType::Buy => {
							Self::do_partially_fulfill_order_buy(&order_info, &fulfiller, amount)?;
							Self::do_fulfill_order_buy(&order_info, &order_id)?;
						},
						OrderType::Sell => {
							Self::do_partially_fulfill_order_sell(&order_info, &fulfiller, amount)?;
							Self::do_fulfill_order_sell(&order_info, &order_id)?;
						},
					}

					Self::deposit_event(Event::OrderFulfilled {
						id: order_id.clone(),
						who: fulfiller,
					});

					OrderSMTsAmount::<T, I>::remove(order_id.clone());
					OrderBook::<T, I>::remove(order_id);
				},
				Ordering::Less => {
					let residual = available_tokens_amount - amount;
					match order_info.order_type {
						OrderType::Buy => {
							Self::do_partially_fulfill_order_buy(&order_info, &fulfiller, amount)?;
						},
						OrderType::Sell => {
							Self::do_partially_fulfill_order_sell(&order_info, &fulfiller, amount)?;
						},
					}
					OrderSMTsAmount::<T, I>::try_mutate(order_id.clone(), |vec_data| {
						vec_data
							.try_push((fulfiller.clone(), amount))
							.map_err(|_| Error::<T, I>::ExceedsMaxFulfillersCount)
					})?;

					Self::deposit_event(Event::OrderPartiallyFulfilled {
						id: order_id,
						who: fulfiller,
						amount,
						residual,
					});
				},
				_ => return Err(Error::<T, I>::InvalidTokensAmount.into()),
			}
			Ok(())
		}
	}

	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		pub fn get_next_order_id() -> Result<T::OrderId, DispatchError> {
			use num_traits::{bounds::Bounded, identities::One};
			<NextOrderId<T, I>>::try_mutate(|n| {
				let id = n.clone();
				ensure!(
					id != <T as pallet::Config<I>>::OrderId::max_value(),
					Error::<T, I>::NoAvailableOrderId
				);
				*n += T::OrderId::one();
				Ok(id)
			})
		}

		fn do_fulfill_order_buy(
			order_info: &OrderInfoOf<T>,
			order_id: &T::OrderId,
		) -> DispatchResult {
			let amount_to_unlock = order_info.token_amount * order_info.price_per_token;
			<T as pallet_insurances::Config>::StableCurrency::increase_balance(
				<T as pallet_insurances::Config>::UsdtId::get(),
				&order_info.creator,
				amount_to_unlock,
				Precision::Exact,
			)?;

			let order_data: Vec<_> = OrderSMTsAmount::<T, I>::get(order_id).into_iter().collect();

			for (account_id, amount) in order_data {
				let amount_to_pay = amount * order_info.price_per_token;
				<T as pallet_insurances::Config>::StableCurrency::transfer(
					<T as pallet_insurances::Config>::UsdtId::get(),
					&order_info.creator,
					&account_id,
					amount_to_pay,
					Preservation::Preserve,
				)?;
			}

			T::SecondaryMarketToken::transfer(
				order_info.token_id.clone(),
				&PalletAccountId::<T, I>::get().unwrap(),
				&order_info.creator,
				order_info.token_amount,
				Preservation::Expendable,
			)?;

			Ok(())
		}

		fn do_fulfill_order_sell(
			order_info: &OrderInfoOf<T>,
			order_id: &T::OrderId,
		) -> DispatchResult {
			let order_data: Vec<_> = OrderSMTsAmount::<T, I>::get(order_id).into_iter().collect();

			for (fulfiller, amount) in order_data {
				let amount_to_pay = amount * order_info.price_per_token;
				<T as pallet_insurances::Config>::StableCurrency::increase_balance(
					<T as pallet_insurances::Config>::UsdtId::get(),
					&fulfiller,
					amount_to_pay,
					Precision::Exact,
				)?;

				T::SecondaryMarketToken::transfer(
					order_info.token_id.clone(),
					&PalletAccountId::<T, I>::get().unwrap(),
					&fulfiller,
					amount,
					Preservation::Expendable,
				)?;

				<T as pallet_insurances::Config>::StableCurrency::transfer(
					<T as pallet_insurances::Config>::UsdtId::get(),
					&fulfiller,
					&order_info.creator,
					amount_to_pay,
					Preservation::Preserve,
				)?;
			}

			Ok(())
		}

		fn do_partially_fulfill_order_buy(
			order_info: &OrderInfoOf<T>,
			fulfiller: &T::AccountId,
			amount: <T as pallet_insurances::Config>::Balance,
		) -> DispatchResult {
			T::SecondaryMarketToken::transfer(
				order_info.token_id.clone(),
				fulfiller,
				&PalletAccountId::<T, I>::get().unwrap(),
				amount,
				Preservation::Expendable,
			)?;

			Ok(())
		}

		fn do_partially_fulfill_order_sell(
			order_info: &OrderInfoOf<T>,
			fulfiller: &T::AccountId,
			amount: <T as pallet_insurances::Config>::Balance,
		) -> DispatchResult {
			let amount_to_pay = amount * order_info.price_per_token;

			ensure!(
				<T as pallet_insurances::Config>::StableCurrency::can_withdraw(
					<T as pallet_insurances::Config>::UsdtId::get(),
					fulfiller,
					amount_to_pay,
				) == WithdrawConsequence::Success,
				Error::<T, I>::NotEnoughFunds
			);

			<T as pallet_insurances::Config>::StableCurrency::decrease_balance(
				<T as pallet_insurances::Config>::UsdtId::get(),
				fulfiller,
				amount_to_pay,
				Precision::Exact,
				Preservation::Preserve,
				Fortitude::Polite,
			)
			.or(Err(Error::<T, I>::NotEnoughFunds))?;

			Ok(())
		}

		fn do_cancel_order(creator: T::AccountId, order_id: T::OrderId) -> DispatchResult {
			let order_info =
				OrderBook::<T, I>::get(order_id.clone()).ok_or(Error::<T, I>::InvalidOrderId)?;

			match order_info.order_type {
				OrderType::Buy => {
					let order_data: Vec<_> =
						OrderSMTsAmount::<T, I>::get(order_id.clone()).into_iter().collect();

					for (fulfiller, amount) in order_data {
						T::SecondaryMarketToken::transfer(
							order_info.token_id.clone(),
							&PalletAccountId::<T, I>::get().unwrap(),
							&fulfiller,
							amount,
							Preservation::Expendable,
						)?;
					}
					let amount_to_unlock = order_info.token_amount * order_info.price_per_token;
					<T as pallet_insurances::Config>::StableCurrency::increase_balance(
						<T as pallet_insurances::Config>::UsdtId::get(),
						&creator,
						amount_to_unlock,
						Precision::Exact,
					)?;
				},
				OrderType::Sell => {
					let order_data: Vec<_> =
						OrderSMTsAmount::<T, I>::get(order_id.clone()).into_iter().collect();

					for (fulfiller, amount) in order_data {
						let amount_to_unlock = amount * order_info.price_per_token;
						<T as pallet_insurances::Config>::StableCurrency::increase_balance(
							<T as pallet_insurances::Config>::UsdtId::get(),
							&fulfiller,
							amount_to_unlock,
							Precision::Exact,
						)?;
					}

					T::SecondaryMarketToken::transfer(
						order_info.token_id,
						&PalletAccountId::<T, I>::get().unwrap(),
						&creator,
						order_info.token_amount,
						Preservation::Expendable,
					)?;
				},
			}

			OrderSMTsAmount::<T, I>::remove(order_id.clone());
			OrderBook::<T, I>::remove(order_id.clone());
			Self::deposit_event(Event::OrderCanceled { id: order_id });

			Ok(())
		}

		pub fn do_clean_orders(
			token_id: <T as pallet_insurances::Config>::AssetId,
		) -> DispatchResult {
			for (order_id, order_info) in OrderBook::<T, I>::iter() {
				if order_info.token_id == token_id.clone() {
					Self::do_cancel_order(order_info.creator, order_id)?;
				}
			}

			Ok(())
		}
	}
}
