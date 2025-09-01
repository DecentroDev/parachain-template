use crate as pallet_insurances;
use frame_support::{
	pallet_prelude::*,
	parameter_types,
	traits::{AsEnsureOriginWithArg, ConstU16, ConstU64, SortedMembers},
	PalletId,
};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};
use system::{EnsureRoot, EnsureSigned};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type AccountId = u32;
type Balance = u64;
type AssetId = u64;

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
		Insurances: pallet_insurances::{Pallet, Storage, Event<T>, Config<T>},
	}
);

impl system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
	pub const MaxReserves: u32 = 50;
	pub const MaxHolds: u32 = 50;
	pub const MaxFreezes: u32 = 0;
}
impl pallet_balances::Config for Test {
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Pallet<Test>;
	type MaxLocks = ();
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
	pub const AssetAccountDeposit: Balance = 0;
	pub const MetadataDepositBase: Balance = 0;
	pub const MetadataDepositPerByte: Balance = 0;
	pub const ApprovalDeposit: Balance = 0;
	pub const AssetsStringLimit: u32 = 32;
	pub const RemoveItemsLimit: u32 = 1000;
}

impl pallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type RemoveItemsLimit = RemoveItemsLimit;
	type AssetId = AssetId;
	type AssetIdParameter = codec::Compact<u64>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = AssetAccountDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const CollectionDeposit: Balance = 0;
	pub const ItemDeposit: Balance = 0;
	pub const KeyLimit: u32 = 32;
	pub const ValueLimit: u32 = 256;
	pub const UniquesStringLimit: u32 = 50;
}

impl pallet_uniques::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = u64;
	type ItemId = u64;
	type Currency = Balances;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type Locker = ();
	type CollectionDeposit = CollectionDeposit;
	type ItemDeposit = ItemDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type AttributeDepositBase = MetadataDepositBase;
	type DepositPerByte = MetadataDepositPerByte;
	type StringLimit = UniquesStringLimit;
	type KeyLimit = KeyLimit;
	type ValueLimit = ValueLimit;
	// #[cfg(feature = "runtime-benchmarks")]
	// type Helper = ();
	type WeightInfo = pallet_uniques::weights::SubstrateWeight<Test>;
}

parameter_types! {
	pub const AssetMintCount: u64 = 100;
	pub const AssetMinBalance: u64 = 1;
	pub const InsurancesStringLimit: u32 = 50;
	pub const ZeroAddressId: PalletId = PalletId(*b"pr/mxzer");
	pub const UsdtId: AssetId = 1;
}

pub struct OneToFive;
impl SortedMembers<u64> for OneToFive {
	fn sorted_members() -> Vec<u64> {
		vec![1, 2, 3, 4, 5]
	}
	// #[cfg(feature = "runtime-benchmarks")]
	// fn add(_m: &u64) {}
}

impl SortedMembers<u32> for OneToFive {
	fn sorted_members() -> Vec<u32> {
		vec![1, 2, 3, 4, 5]
	}
	// #[cfg(feature = "runtime-benchmarks")]
	// fn add(_m: &u32) {}
}

impl pallet_insurances::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type SecondaryMarketToken = Assets;
	type InsuredToken = Uniques;
	type StableCurrency = Assets;
	type UsdtId = UsdtId;
	type Balance = u64;
	type AssetId = AssetId;
	type CurrencyId = AssetId;
	type AssetMinBalance = AssetMinBalance;
	type StringLimit = InsurancesStringLimit;
	type NftId = AssetId;
	type ZeroAddressId = ZeroAddressId;
}

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = system::GenesisConfig::default().build_storage::<Test>().unwrap().into();
	pallet_insurances::GenesisConfig::<Test>::default()
		.assimilate_storage(&mut t)
		.unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
