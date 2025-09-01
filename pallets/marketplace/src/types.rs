use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

#[derive(Clone, Debug, Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq)]
pub enum OrderType {
	Buy,
	Sell,
}

#[derive(Clone, Debug, Encode, Decode, TypeInfo, PartialEq, Eq, MaxEncodedLen)]
pub struct OrderInfo<AccountId, TokenId, Balance> {
	pub creator: AccountId,
	pub order_type: OrderType,
	pub token_id: TokenId,
	pub token_amount: Balance,
	pub price_per_token: Balance,
}
