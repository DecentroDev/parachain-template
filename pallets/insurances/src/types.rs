use frame_support::pallet_prelude::*;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use codec::{Decode, Encode, DecodeWithMemTracking, MaxEncodedLen};

pub type ContractLink<T, P> = BoundedVec<T, P>;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Eq, PartialEq, MaxEncodedLen, TypeInfo, Debug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct InsuranceMetadata<Balance, AccountId, BlockNumber, AssetId, ContractLink> {
	pub name: InsuranceType,
	pub location: u8,
	pub creator: AccountId,
	pub status: InsuranceStatus,
	pub underwrite_amount: Balance,
	pub premium_amount: Balance,
	pub contract_link: ContractLink,
	pub starts_on: BlockNumber,
	pub ends_on: BlockNumber,
	pub smt_id: Option<AssetId>,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Eq, PartialEq, MaxEncodedLen, TypeInfo, Debug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct InsuranceOffer<Balance, BlockNumber> {
	pub location: u8,
	pub insurance_type: InsuranceType,
	pub starts_on: BlockNumber,
	pub ends_on: BlockNumber,
	pub underwrite_amount: Balance,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Eq, PartialEq, MaxEncodedLen, TypeInfo, Debug, Ord, PartialOrd)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum InsuranceType {
	Cyclone,
	Earthquake,
	Rainfall,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Eq, PartialEq, MaxEncodedLen, TypeInfo, Debug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum InsuranceStatus {
	Active,
	Expired,
	NotStarted,
	PaidOut,
	PayoutPending,
	PremiumPayoutPending,
}
