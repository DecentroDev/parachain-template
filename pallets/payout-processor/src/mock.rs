use crate as pallet_payout_processor;
use std::str::FromStr;

use frame_support::{
	pallet_prelude::Weight,
	parameter_types,
	traits::{AsEnsureOriginWithArg, ConstU16, ConstU32, ConstU64, SortedMembers, Time},
	PalletId,
};
use frame_system as system;
use orml_oracle::DefaultCombineData;
use pallet_collective::PrimeDefaultVote;
use pallet_payout_processor::types::{InsuranceEventHandler, InsuranceType};
use sp_core::{
	sr25519::{Public, Signature},
	H256,
};
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{BlakeTwo256, Extrinsic as ExtrinsicT, IdentifyAccount, IdentityLookup, Verify},
};
use system::{EnsureRoot, EnsureSigned};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u128;
type AssetId = u32;
type OrderId = u64;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Oracle: orml_oracle::<Instance1>::{Event<T>},
		Collective: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>},
		Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
		Uniques: pallet_uniques::{Pallet, Call, Storage, Event<T>},
		Insurances: pallet_insurances,
		Dao: pallet_dao::<Instance1>,
		PayoutProcessor: pallet_payout_processor::<Instance1>::{Pallet, Event<T>, Call},
		Marketplace: pallet_marketplace::<Instance1>,
	}
);

parameter_types! {
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(Weight::MAX);
}

impl system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u32;
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
	pub const CollectionDeposit: Balance = 0;
	pub const ItemDeposit: Balance = 0;
	pub const KeyLimit: u32 = 32;
	pub const ValueLimit: u32 = 256;
	pub const UniquesStringLimit: u32 = 50;
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
	type StringLimit = UniquesStringLimit;
	type KeyLimit = KeyLimit;
	type ValueLimit = ValueLimit;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
	type WeightInfo = pallet_uniques::weights::SubstrateWeight<Test>;
}

thread_local! {
	static TIME: core::cell::RefCell<u32>  = core::cell::RefCell::new(0);
}

pub struct Timestamp;
impl Time for Timestamp {
	type Moment = u32;

	fn now() -> Self::Moment {
		TIME.with(|v| *v.borrow())
	}
}

impl Timestamp {
	pub fn set_timestamp(val: u32) {
		TIME.with(|v| *v.borrow_mut() = val);
	}
}

parameter_types! {
	pub RootOperatorAccountId: AccountId = test_pub(4);
	pub static OracleMembers: Vec<AccountId> = vec![
		test_pub(1),
		test_pub(2),
		test_pub(3),
		test_pub(4),
	];
}

pub struct Members;

impl SortedMembers<AccountId> for Members {
	fn sorted_members() -> Vec<AccountId> {
		OracleMembers::get()
	}
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
	type AssetIdParameter = u32;
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
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
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
	type StringLimit = AssetsStringLimit;
	type AssetMinBalance = AssetMinBalance;
	type ZeroAddressId = ZeroAddressId;
}

impl orml_oracle::Config<orml_oracle::Instance1> for Test {
	type RuntimeEvent = RuntimeEvent;
	type OnNewData = InsuranceEventHandler<Self, orml_oracle::Instance1>;
	type CombineData = DefaultCombineData<Self, ConstU32<3>, ConstU32<600>, orml_oracle::Instance1>;
	type Time = Timestamp;
	type OracleKey = (InsuranceType, u8);
	type OracleValue = Option<(
		<Test as pallet_insurances::Config>::NftId,
		<Test as pallet_insurances::Config>::NftId,
	)>;
	type RootOperatorAccountId = RootOperatorAccountId;
	type Members = Members;
	type WeightInfo = ();
	type MaxHasDispatchedSize = ConstU32<100>;
}

type Extrinsic = TestXt<RuntimeCall, ()>;
type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

impl frame_system::offchain::SigningTypes for Test {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	type OverarchingCall = RuntimeCall;
	type Extrinsic = Extrinsic;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall,
		_public: <Signature as Verify>::Signer,
		_account: AccountId,
		nonce: <Test as frame_system::Config>::Index,
	) -> Option<(RuntimeCall, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
		Some((call, (nonce, ())))
	}
}

parameter_types! {
	pub DaoMotionDuration: u64 = 2;
	pub const DaoMaxProposals: u32 = 100;
	pub const DaoMaxMembers: u32 = 100;
	pub MaxProposalWeight: Weight = sp_runtime::Perbill::from_percent(80) * BlockWeights::get().max_block;
}

type DaoCollective = pallet_collective::Instance1;
impl pallet_collective::Config<DaoCollective> for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = ConstU64<3>;
	type MaxProposals = DaoMaxProposals;
	type MaxMembers = DaoMaxMembers;
	type DefaultVote = PrimeDefaultVote;
	type WeightInfo = ();
	type SetMembersOrigin = EnsureRoot<Self::AccountId>;
	type MaxProposalWeight = MaxProposalWeight;
}

parameter_types! {
	pub const Quorum: u32 = 1;
	pub const DaoMaxProposalWeight: u64 = 1_000_000_000_000_000;
	pub const MaxLengthBound: u32 = 1_000_000_000;
	pub const DaoPalletId: PalletId = PalletId(*b"pr/mxdao");
}

impl pallet_dao::Config<pallet_dao::Instance1> for Test {
	type DaoOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, DaoCollective, 1, 2>;
	type AuthorityId = pallet_dao::crypto::AuthId;
	type Call = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type LocalCurrency = Balances;
	type Quorum = Quorum;
	type MaxProposalWeight = DaoMaxProposalWeight;
	type MaxLengthBound = MaxLengthBound;
	type PalletId = DaoPalletId;
	type WeightInfo = ();
}

parameter_types! {
	pub const MarketplacePalletId: PalletId = PalletId(*b"pr/mxmpl");
	// Set the base price to 100 tokens
	pub const BaseSecondaryMarketTokenPrice: Balance = 100;
	pub const MaxFulfillers: u32 = 2;
}

impl pallet_marketplace::Config<pallet_marketplace::Instance1> for Test {
	type RuntimeEvent = RuntimeEvent;
	type OrderId = OrderId;
	type WeightInfo = ();
	type Currency = Balances;
	type DaoAccountIdProvider = Dao;
	type PalletId = MarketplacePalletId;
	type BaseSecondaryMarketTokenPrice = BaseSecondaryMarketTokenPrice;
	type MaxFulfillers = MaxFulfillers;
}

impl pallet_payout_processor::Config<pallet_payout_processor::Instance1> for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type DaoAccountIdProvider = Dao;
	type BaseSecondaryMarketTokenPrice = BaseSecondaryMarketTokenPrice;
	type WeightInfo = ();
}

fn test_pub(id: u8) -> sp_core::sr25519::Public {
	sp_core::sr25519::Public::from_raw([id; 32])
}

parameter_types! {
	pub const ALICE: AccountId = test_pub(1);
	pub const BOB: AccountId = test_pub(2);
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	use frame_support::traits::GenesisBuild;
	let mut t = system::GenesisConfig::default().build_storage::<Test>().unwrap().into();
	pallet_dao::GenesisConfig::<Test, _>::default()
		.assimilate_storage(&mut t)
		.unwrap();
	pallet_insurances::GenesisConfig::<Test>::default()
		.assimilate_storage(&mut t)
		.unwrap();
	pallet_marketplace::GenesisConfig::<Test, _>::default()
		.assimilate_storage(&mut t)
		.unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
