// Custom pallet configurations for the parachain template runtime

use crate::*;

// DAO Pallet Configuration (temporarily disabled)
// impl pallet_dao::Config for Runtime {
// 	type RuntimeEvent = RuntimeEvent;
// 	type RuntimeCall = RuntimeCall;
// 	type WeightInfo = ();
// }

// Insurances Pallet Configuration (temporarily disabled)
// impl pallet_insurances::Config for Runtime {
// 	type RuntimeEvent = RuntimeEvent;
// 	type WeightInfo = ();
// }

// Marketplace Pallet Configuration (temporarily disabled)
// impl pallet_marketplace::Config for Runtime {
// 	type RuntimeEvent = RuntimeEvent;
// 	type WeightInfo = ();
// }

// Payout Processor Pallet Configuration
impl pallet_payout_processor::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type WeightInfo = pallet_payout_processor::weights::PayoutProcessorWeight<Runtime>;
}
