//! Runtime API definition for oracle module.

#![cfg_attr(not(feature = "std"), no_std)]
use codec::Codec;

sp_api::decl_runtime_apis! {
	pub trait PayoutProcessorApi<Event, Location, Moment> where
		Event: Codec,
		Location: Codec,
		Moment: Codec
	{
		fn get_event(event: (Event, Location)) -> Option<Moment>;
	}
}
