#![allow(unused)]
use std::sync::Arc;

use codec::Codec;
use jsonrpsee::{
	core::{async_trait, RpcResult},
	proc_macros::rpc,
	types::{error::CallError, ErrorObject},
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

pub use pallet_dao_rpc_runtime_api::DaoApi as DaoRuntimeApi;

#[rpc(client, server)]
pub trait DaoApi<BlockHash, AccountId> {
	#[method(name = "dao_isDaoMember")]
	fn is_dao_member(&self, account_id: AccountId, at: Option<BlockHash>) -> RpcResult<bool>;
}

/// Provides RPC methods to query dao value.
pub struct Dao<C, P> {
	/// Shared reference to the client.
	client: Arc<C>,
	_marker: std::marker::PhantomData<P>,
}

impl<C, P> Dao<C, P> {
	/// Creates a new instance of the `Dao` helper.
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
impl<C, Block, AccountId> DaoApiServer<<Block as BlockT>::Hash, AccountId> for Dao<C, Block>
where
	Block: BlockT,
	C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
	C::Api: DaoRuntimeApi<Block, AccountId>,
	AccountId: Codec,
{
	fn is_dao_member(
		&self,
		account_id: AccountId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<bool> {
		let api = self.client.runtime_api();
		let at_hash = at.unwrap_or_else(|| self.client.info().best_hash);

		api.is_dao_member(at_hash, account_id).map_err(|e| {
			CallError::Custom(ErrorObject::owned(
				Error::RuntimeError.into(),
				"Unable to check member.",
				Some(e.to_string()),
			))
			.into()
		})
	}
}
