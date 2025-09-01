#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;
// use serde::{Deserialize};

/// A number of members.
///
/// This also serves as a number of voting members, and since for motions, each member may
/// vote exactly once, therefore also the number of votes for any given motion.
pub type MemberCount = u32;

#[derive(Clone, Encode, Decode, TypeInfo)]
pub struct VotingMetadata<BlockNumber, AccountId, Balance, Hash, ProposalIndex> {
	pub ends_on: BlockNumber,
	pub beneficiary: AccountId,
	pub premium_amount: Balance,
	pub metadata_hash: Hash,
	pub proposal_index: ProposalIndex,
}

#[derive(Clone, Debug, Encode, Decode, TypeInfo, PartialEq)]
pub struct GetPremiumParams<Balance> {
	pub latitude: Balance,
	pub longtitude: Balance,
	pub start_date: Balance,
	pub duration_in_hours: Balance,
	pub threshold: Balance,
	pub coverage: Balance,
	pub number_of_simulations: Balance,
	pub roc: Balance,
}

#[derive(Debug)]
pub struct PriceJson<'a> {
	pub avg_cost: &'a str,
	pub required_capital: &'a str,
	pub diversified_capital: &'a str,
	pub recommended_premium: u128,
	pub closest_point: &'a str,
	pub dist_closest_point_km: &'a str,
}
