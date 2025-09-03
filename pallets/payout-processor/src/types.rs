use frame_support::pallet_prelude::*;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// Insurance types for the payout processor.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum InsuranceType {
	Weather,
	NaturalDisaster,
	Other,
}

/// Insurance reasons for payouts.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum InsuranceReason {
	Expired,
	EventOccurred,
	Other,
}

/// Insurance event data.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct InsuranceEvent {
	pub insurance_type: InsuranceType,
	pub location_id: u8,
	pub severity: u32,
	pub timestamp: u64,
}

