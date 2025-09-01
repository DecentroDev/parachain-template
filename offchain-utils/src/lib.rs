#![cfg_attr(not(feature = "std"), no_std)]

pub mod offchain_api_key;

pub use offchain_api_key::{DefaultOffchainApiKey, OffchainApiKey};
