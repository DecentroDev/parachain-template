//! Runtime API definition for insurance module.

#![cfg_attr(not(feature = "std"), no_std)]
use codec::Codec;

sp_api::decl_runtime_apis! {
	pub trait DaoApi<AccountId> where
		AccountId: Codec
	{
		fn is_dao_member(account_id: AccountId) -> bool;
	}
}
