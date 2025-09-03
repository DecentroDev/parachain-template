#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod types;
pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, ReservableCurrency},
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::Zero;
	use sp_std::vec::Vec;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The currency mechanism.
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	/// This storage contains the latest known total number of active insurances.
	#[pallet::storage]
	#[pallet::getter(fn insurance_count)]
	pub type InsuranceCount<T> = StorageValue<_, u64, ValueQuery>;

	/// Event thresholds for different insurance types.
	#[pallet::storage]
	#[pallet::getter(fn event_thresholds)]
	pub type EventThresholds<T> = StorageMap<
		_,
		Blake2_128Concat,
		u8, // Insurance type ID
		u32, // Threshold value
		ValueQuery,
	>;

	/// Location coordinates for insurance events.
	#[pallet::storage]
	#[pallet::getter(fn location_coordinates)]
	pub type LocationCoordinates<T> = StorageMap<
		_,
		Blake2_128Concat,
		u8,         // Location ID
		(i32, i32), // Latitude and longitude multiplied by 10000
		OptionQuery,
	>;

	/// Location names for insurance events.
	#[pallet::storage]
	#[pallet::getter(fn location_names)]
	pub type LocationNames<T> = StorageMap<
		_,
		Blake2_128Concat,
		u8,                           // Location ID
		BoundedVec<u8, ConstU32<64>>, // Location name as bounded bytes
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Insurance count updated.
		InsuranceCountUpdated { count: u64 },
		/// Event threshold set.
		EventThresholdSet { insurance_type: u8, threshold: u32 },
		/// Location coordinates set.
		LocationCoordinatesSet { location_id: u8, lat: i32, lon: i32 },
		/// Location name set.
		LocationNameSet { location_id: u8, name: BoundedVec<u8, ConstU32<64>> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Invalid threshold value.
		InvalidThreshold,
		/// Location not found.
		LocationNotFound,
		/// Name too long.
		NameTooLong,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set the insurance count.
		#[pallet::weight(T::WeightInfo::set_insurance_count())]
		pub fn set_insurance_count(origin: OriginFor<T>, count: u64) -> DispatchResult {
			let _ = ensure_root(origin)?;
			
			InsuranceCount::<T>::put(count);
			Self::deposit_event(Event::InsuranceCountUpdated { count });
			
			Ok(())
		}

		/// Set event threshold for an insurance type.
		#[pallet::weight(T::WeightInfo::set_event_threshold())]
		pub fn set_event_threshold(
			origin: OriginFor<T>,
			insurance_type: u8,
			threshold: u32,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;
			
			ensure!(threshold > 0, Error::<T>::InvalidThreshold);
			
			EventThresholds::<T>::insert(insurance_type, threshold);
			Self::deposit_event(Event::EventThresholdSet { insurance_type, threshold });
			
			Ok(())
		}

		/// Set location coordinates.
		#[pallet::weight(T::WeightInfo::set_location_coordinates())]
		pub fn set_location_coordinates(
			origin: OriginFor<T>,
			location_id: u8,
			lat: i32,
			lon: i32,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;
			
			LocationCoordinates::<T>::insert(location_id, (lat, lon));
			Self::deposit_event(Event::LocationCoordinatesSet { location_id, lat, lon });
			
			Ok(())
		}

		/// Set location name.
		#[pallet::weight(T::WeightInfo::set_location_name())]
		pub fn set_location_name(
			origin: OriginFor<T>,
			location_id: u8,
			name: Vec<u8>,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;
			
			let bounded_name: BoundedVec<u8, ConstU32<64>> = name
				.try_into()
				.map_err(|_| Error::<T>::NameTooLong)?;
			
			LocationNames::<T>::insert(location_id, bounded_name.clone());
			Self::deposit_event(Event::LocationNameSet { location_id, name: bounded_name });
			
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			T::WeightInfo::on_initialize()
		}

		fn on_finalize(_n: BlockNumberFor<T>) {
			// Clean up logic can go here
		}
	}

	impl<T: Config> Pallet<T> {
		/// Get the current insurance count.
		pub fn get_insurance_count() -> u64 {
			InsuranceCount::<T>::get()
		}

		/// Get event threshold for an insurance type.
		pub fn get_event_threshold(insurance_type: u8) -> Option<u32> {
			Some(EventThresholds::<T>::get(insurance_type))
		}

		/// Get location coordinates.
		pub fn get_location_coordinates(location_id: u8) -> Option<(i32, i32)> {
			LocationCoordinates::<T>::get(location_id)
		}

		/// Get location name.
		pub fn get_location_name(location_id: u8) -> Option<BoundedVec<u8, ConstU32<64>>> {
			LocationNames::<T>::get(location_id)
		}
	}
}
