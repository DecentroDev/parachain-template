use std::sync::Arc;

use codec::Codec;
use jsonrpsee::{
	core::{async_trait, RpcResult},
	proc_macros::rpc,
	types::{error::CallError, ErrorObject},
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;

pub use pallet_insurances_rpc_runtime_api::InsurancesApi as InsurancesRuntimeApi;

#[rpc(client, server)]
pub trait InsurancesApi<BlockHash, AccountId, AssetId, Metadata> {
	#[method(name = "insurances_getUserMetadata")]
	fn get_user_metadata(
		&self,
		collection_id: AssetId,
		item_id: AssetId,
		at: Option<BlockHash>,
	) -> RpcResult<Option<Metadata>>;

	#[method(name = "insurances_getUserAssetsInfo")]
	fn get_user_assets_info(
		&self,
		account_id: AccountId,
		at: Option<BlockHash>,
	) -> RpcResult<Option<Vec<(AssetId, AssetId)>>>;
}

/// Provides RPC methods to query insurance values.
pub struct Insurances<C, P> {
	/// Shared reference to the client.
	client: Arc<C>,
	_marker: std::marker::PhantomData<P>,
}

impl<C, P> Insurances<C, P> {
	/// Creates a new instance of the `Insurances` helper.
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

pub enum Error {
	RuntimeError,
}

impl From<Error> for i32 {
	fn from(e: Error) -> i32 {
		match e {
			Error::RuntimeError => 1,
		}
	}
}

#[async_trait]
impl<C, Block, AccountId, AssetId, Metadata>
	InsurancesApiServer<<Block as BlockT>::Hash, AccountId, AssetId, Metadata> for Insurances<C, Block>
where
	Block: BlockT,
	C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
	C::Api: InsurancesRuntimeApi<Block, AccountId, AssetId, Metadata>,
	AccountId: Codec,
	AssetId: Codec,
	Metadata: Codec,
{
	fn get_user_metadata(
		&self,
		collection_id: AssetId,
		item_id: AssetId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Option<Metadata>> {
		let api = self.client.runtime_api();
		let at_hash = at.unwrap_or_else(|| self.client.info().best_hash);

		api.get_user_metadata(at_hash, collection_id, item_id).map_err(|e| {
			CallError::Custom(ErrorObject::owned(
				Error::RuntimeError.into(),
				"Unable to get metadata",
				Some(e.to_string()),
			))
			.into()
		})
	}

	fn get_user_assets_info(
		&self,
		account_id: AccountId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Option<Vec<(AssetId, AssetId)>>> {
		let api = self.client.runtime_api();
		let at_hash = at.unwrap_or_else(|| self.client.info().best_hash);

		api.get_user_assets_info(at_hash, account_id).map_err(|e| {
			CallError::Custom(ErrorObject::owned(
				Error::RuntimeError.into(),
				"Unable to get user assets",
				Some(e.to_string()),
			))
			.into()
		})
	}
}
