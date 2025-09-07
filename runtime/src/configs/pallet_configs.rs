// Custom pallet configurations for the parachain template runtime

use crate::*;
use frame_support::traits::{ConstU32, ConstU128};
use sp_runtime::traits::AccountIdConversion;
use frame_support::{parameter_types, PalletId};

// Zero address pallet ID for insurances
parameter_types! {
	pub const ZeroAddressPalletId: PalletId = PalletId(*b"pr/mxzer");
}

// DAO Pallet Configuration (temporarily disabled)
// impl pallet_dao::Config for Runtime {
// 	type RuntimeEvent = RuntimeEvent;
// 	type RuntimeCall = RuntimeCall;
// 	type WeightInfo = ();
// }

// Insurances Pallet Configuration (temporarily disabled due to configuration issues)
impl pallet_insurances::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_insurances::weights::InsurancesWeight<Runtime>;
	type InsuredToken = pallet_uniques::Pallet<Runtime>;
	type SecondaryMarketToken = pallet_assets::Pallet<Runtime>;
	type NftId = u32;
	type AssetId = u32;
	type Balance = Balance;
	type StringLimit = ConstU32<50>;
	type AssetMinBalance = ConstU128<1>;
	type CurrencyId = u32;
	type StableCurrency = pallet_assets::Pallet<Runtime>;
	type UsdtId = frame_support::traits::ConstU32<1>;
	type ZeroAddressId = ZeroAddressPalletId;
}

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
