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

pub use pallet_payout_processor_rpc_runtime_api::PayoutProcessorApi as PayoutProcessorRuntimeApi;

#[rpc(client, server)]
pub trait PayoutProcessorApi<BlockHash, Event, Location, Moment> {
	#[method(name = "payoutProcessor_getEvent")]
	fn get_event(
		&self,
		event: (Event, Location),
		at: Option<BlockHash>,
	) -> RpcResult<Option<Moment>>;
}

/// Provides RPC methods to query payout processor value.
pub struct PayoutProcessor<C, B> {
	/// Shared reference to the client.
	client: Arc<C>,
	_marker: std::marker::PhantomData<B>,
}

impl<C, B> PayoutProcessor<C, B> {
	/// Creates a new instance of the `PayoutProcessor` helper.
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
impl<C, Block, Event, Location, Moment>
	PayoutProcessorApiServer<<Block as BlockT>::Hash, Event, Location, Moment>
	for PayoutProcessor<C, Block>
where
	Block: BlockT,
	C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
	C::Api: PayoutProcessorRuntimeApi<Block, Event, Location, Moment>,
	Event: Codec,
	Location: Codec,
	Moment: Codec,
{
	fn get_event(
		&self,
		event: (Event, Location),
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Option<Moment>> {
		let api = self.client.runtime_api();
		let at_hash = at.unwrap_or_else(|| self.client.info().best_hash);

		api.get_event(at_hash, event).map_err(|e| {
			CallError::Custom(ErrorObject::owned(
				Error::RuntimeError.into(),
				"Unable to get event.",
				Some(e.to_string()),
			))
			.into()
		})
	}
}
