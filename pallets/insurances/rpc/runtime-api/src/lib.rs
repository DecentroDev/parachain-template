//! Runtime API definition for insurances module.

#![cfg_attr(not(feature = "std"), no_std)]
use codec::Codec;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
	pub trait InsurancesApi<AccountId, AssetId, Metadata> where
		AccountId: Codec,
		AssetId: Codec,
		Metadata: Codec,
	{
		fn get_user_metadata(collection_id: AssetId, item_id: AssetId) -> Option<Metadata>;
		fn get_user_assets_info(account_id: AccountId) -> Option<Vec<(AssetId, AssetId)>>;
	}
}
