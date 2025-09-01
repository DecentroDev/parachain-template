use crate as pallet_marketplace;
use frame_support::{
	parameter_types,
	traits::{AsEnsureOriginWithArg, ConstU128},
	weights::constants::RocksDbWeight,
	PalletId,
};
use frame_system as system;
use sp_runtime::{
	generic,
	traits::{AccountIdLookup, BlakeTwo256},
};

use system::{EnsureRoot, EnsureSigned};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

type AccountId = u32;
type Balance = u128;
type AssetId = u32;
type OrderId = u64;
type BlockNumber = u64;
type Index = u32;
type Hash = sp_core::H256;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
		Uniques: pallet_uniques::{Pallet, Call, Storage, Event<T>},
		Insurances: pallet_insurances::{Pallet, Storage, Event<T>},
		Collective: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>},
		Dao: pallet_dao::<Instance1>,
		Marketplace: pallet_marketplace::<Instance1>,
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type AccountId = AccountId;
	type RuntimeCall = RuntimeCall;
	type Lookup = AccountIdLookup<AccountId, ()>;
	type Index = Index;
	type BlockNumber = BlockNumber;
	type Hash = Hash;
	type Hashing = BlakeTwo256;
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type BlockHashCount = BlockHashCount;
	type DbWeight = RocksDbWeight;
	type Version = ();
	type PalletInfo = PalletInfo;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type AccountData = pallet_balances::AccountData<Balance>;
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1_000;
	pub const MaxReserves: u32 = 50;
	pub const MaxHolds: u32 = 0;
	pub const MaxFreezes: u32 = 0;
}

impl pallet_balances::Config for Test {
	type MaxLocks = ();
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type HoldIdentifier = ();
	type FreezeIdentifier = ();
	type MaxHolds = MaxHolds;
	type MaxFreezes = MaxFreezes;
}

parameter_types! {
	pub const AssetDeposit: Balance = 0;
	pub const ApprovalDeposit: Balance = 0;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 0;
	pub const MetadataDepositPerByte: Balance = 0;
	pub const RemoveItemsLimit: u32 = 1000;
}

impl pallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type RemoveItemsLimit = RemoveItemsLimit;
	type AssetId = AssetId;
	type AssetIdParameter = u32;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = ConstU128<10>;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub const CollectionDeposit: Balance = 0;
	pub const ItemDeposit: Balance = 0;
	pub const KeyLimit: u32 = 32;
	pub const ValueLimit: u32 = 256;
}

impl pallet_uniques::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type Locker = ();
	type CollectionDeposit = CollectionDeposit;
	type ItemDeposit = ItemDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type AttributeDepositBase = MetadataDepositBase;
	type DepositPerByte = MetadataDepositPerByte;
	type StringLimit = StringLimit;
	type KeyLimit = KeyLimit;
	type ValueLimit = ValueLimit;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const AssetMintCount: u64 = 100;
	pub const AssetMinBalance: u64 = 1;
	pub const InsurancesStringLimit: u32 = 50;
	pub const ZeroAddressId: PalletId = PalletId(*b"pr/mxzer");
	pub const UsdtId: AssetId = 1984;
}

/// Configure the pallet-insurances in pallets/insurances.
impl pallet_insurances::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = AssetId;
	type CurrencyId = AssetId;
	type NftId = AssetId;
	type SecondaryMarketToken = Assets;
	type InsuredToken = Uniques;
	type StableCurrency = Assets;
	type UsdtId = UsdtId;
	type StringLimit = InsurancesStringLimit;
	type AssetMinBalance = AssetMinBalance;
	type ZeroAddressId = ZeroAddressId;
}

parameter_types! {
	pub DaoMotionDuration: u64 = 2;
	pub const DaoMaxProposals: u32 = 100;
	pub const DaoMaxMembers: u32 = 100;
}

type DaoCollective = pallet_collective::Instance1;
impl pallet_collective::Config<DaoCollective> for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = DaoMotionDuration;
	type MaxProposals = DaoMaxProposals;
	type MaxMembers = DaoMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type SetMembersOrigin = EnsureRoot<Self::AccountId>;
	type MaxProposalWeight = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const Quorum: u32 = 1;
	pub const MaxProposalWeight: u64 = 1_000_000_000_000_000;
	pub const MaxLengthBound: u32 = 1_000_000_000;
	pub const DaoPalletId: PalletId = PalletId(*b"pr/mxdao");
}

impl pallet_dao::Config<pallet_dao::Instance1> for Test {
	type DaoOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, DaoCollective, 1, 2>;
	type RuntimeEvent = RuntimeEvent;
	type LocalCurrency = Balances;
	type Quorum = Quorum;
	type MaxProposalWeight = MaxProposalWeight;
	type MaxLengthBound = MaxLengthBound;
	type PalletId = DaoPalletId;
	type WeightInfo = ();
}

parameter_types! {
	pub const MarketplacePalletId: PalletId = PalletId(*b"pr/mxmpl");
	// Set the base price to 10 tokens
	pub const BaseSecondaryMarketTokenPrice: Balance = 10;
	pub const MaxFulfillers: u32 = 3;
}

impl pallet_marketplace::Config<pallet_marketplace::Instance1> for Test {
	type RuntimeEvent = RuntimeEvent;
	type OrderId = OrderId;
	type PalletId = MarketplacePalletId;
	type WeightInfo = ();
	type Currency = Balances;
	type DaoAccountIdProvider = Dao;
	type BaseSecondaryMarketTokenPrice = BaseSecondaryMarketTokenPrice;
	type MaxFulfillers = MaxFulfillers;
}

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	use frame_support::traits::GenesisBuild;
	let mut t = system::GenesisConfig::default().build_storage::<Test>().unwrap().into();
	pallet_dao::GenesisConfig::<Test, _>::default()
		.assimilate_storage(&mut t)
		.unwrap();
	pallet_marketplace::GenesisConfig::<Test, _>::default()
		.assimilate_storage(&mut t)
		.unwrap();
	pallet_insurances::GenesisConfig::<Test>::default()
		.assimilate_storage(&mut t)
		.unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
