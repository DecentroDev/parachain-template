use frame_support::pallet_prelude::*;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use super::{Config, Pallet};

pub use pallet_insurances::types::InsuranceType;

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum InsuranceReason {
	Expired,
	EventOccurred,
}

pub struct InsuranceEventHandler<T, I>(PhantomData<T>, PhantomData<I>);

impl<T: Config<I>, I: 'static>
	orml_oracle::OnNewData<
		<T as frame_system::Config>::AccountId,
		<T as orml_oracle::Config<I>>::OracleKey,
		<T as orml_oracle::Config<I>>::OracleValue,
	> for InsuranceEventHandler<T, I>
where
	T::OracleKey: IsType<(InsuranceType, u8)>,
	T::OracleValue: IsType<
		Option<(<T as pallet_insurances::Config>::NftId, <T as pallet_insurances::Config>::NftId)>,
	>,
{
	fn on_new_data(who: &T::AccountId, key: &T::OracleKey, value: &T::OracleValue) {
		let (event, location): (InsuranceType, u8) = key.clone().into();
		Pallet::<T, I>::handle_insurance_event(
			who,
			event.into_ref(),
			location.into_ref(),
			value.clone().into(),
		);
	}
}