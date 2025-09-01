// Custom pallet configurations for the parachain template runtime

use crate::*;
use polkadot_sdk::{frame_support::traits::ConstU32, frame_system::EnsureRoot};

// DAO Pallet Configuration
impl pallet_dao::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = ();
}

// Insurances Pallet Configuration  
impl pallet_insurances::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
}

// Marketplace Pallet Configuration
impl pallet_marketplace::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
}

// Payout Processor Pallet Configuration
impl pallet_payout_processor::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
}
