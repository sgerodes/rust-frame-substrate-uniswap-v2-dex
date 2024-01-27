#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::fungibles;
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use frame_support::traits::fungible;

pub type AssetIdOf<T> = <<T as Config>::Fungibles as fungibles::Inspect<
	<T as frame_system::Config>::AccountId,
>>::AssetId;

pub type BalanceOf<T> = <<T as Config>::NativeBalance as fungible::Inspect<
	<T as frame_system::Config>::AccountId,
>>::Balance;

pub type AssetBalanceOf<T> = <<T as Config>::Fungibles as fungibles::Inspect<
	<T as frame_system::Config>::AccountId,
>>::Balance;

pub type PoolCompositeIdOf<T> = (AssetIdOf<T>, AssetIdOf<T>);

pub type LpAssetId<T> = AssetIdOf<T>;

#[frame_support::pallet]
pub mod pallet {
	use crate::{AssetBalanceOf, AssetIdOf, BalanceOf, LpAssetId, PoolCompositeIdOf};
	use frame_support::traits::fungibles::Create;
	use frame_support::traits::fungibles::Inspect;
	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungible,
			fungibles::{self, Mutate},
			tokens::Preservation,
		},
		Hashable, PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::{
		AccountIdConversion, CheckedMul, IntegerSquareRoot, One, TrailingZeroInput, Zero,
	};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct AssetPair<T: Config> {
		pub asset_a: AssetIdOf<T>,
		pub asset_b: AssetIdOf<T>,
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct LiquidityPool<T: Config> {
		asset_ids: (AssetIdOf<T>, AssetIdOf<T>),
		balances: (AssetBalanceOf<T>, AssetBalanceOf<T>),
		liquidity_token_id: LpAssetId<T>,
	}

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Type to access the Balances Pallet.
		type NativeBalance: fungible::Inspect<Self::AccountId>
			+ fungible::Mutate<Self::AccountId>
			+ fungible::hold::Inspect<Self::AccountId>
			+ fungible::hold::Mutate<Self::AccountId>
			+ fungible::freeze::Inspect<Self::AccountId>
			+ fungible::freeze::Mutate<Self::AccountId>;

		/// Type to access the Assets Pallet.
		type Fungibles: fungibles::Inspect<Self::AccountId>
			+ fungibles::Mutate<Self::AccountId>
			+ fungibles::Create<Self::AccountId>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;
	}

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		#[serde(skip)]
		_config: core::marker::PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			// Initialize pallet balance for creating and minting LP Tokens
			use frame_support::traits::fungible::*;
			let account_id = T::PalletId::get().into_account_truncating();
			let _ = T::NativeBalance::mint_into(&account_id, 100_000u32.into());
		}
	}

	// The pallet's runtime storage items.
	// https://docs.substrate.io/main-docs/build/runtime-storage/
	#[pallet::storage]
	#[pallet::getter(fn something)]
	// Learn more about declaring storage items:
	// https://docs.substrate.io/main-docs/build/runtime-storage/#declaring-storage-items
	pub type Something<T> = StorageValue<_, u32>;

	#[pallet::storage]
	pub type Pools<T> = StorageMap<_, Blake2_128Concat, PoolCompositeIdOf<T>, LiquidityPool<T>>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		PoolCreated {
			pool_id: PoolCompositeIdOf<T>,
			asset_id_a: AssetIdOf<T>,
			asset_id_b: AssetIdOf<T>,
			creator: T::AccountId,
			liquidity_token_id: LpAssetId<T>,
			// block_number: T::BlockNumber,
			// initial_balances: (AssetBalanceOf<T>, AssetBalanceOf<T>),
		},
		LiquiditySupplied {
			pool_id: PoolCompositeIdOf<T>,
			liquidity_token_id: LpAssetId<T>,
			asset_id_a: AssetIdOf<T>,
			asset_id_b: AssetIdOf<T>,
			amount_a: AssetBalanceOf<T>,
			amount_b: AssetBalanceOf<T>,
			liquidity_token_minted: AssetBalanceOf<T>,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Can not create a pool that already exists.
		DuplicatePoolError,
		/// Assets in the pool must be distinct.
		DistinctAssetsRequired,
		/// Trying to do an operation on a pool that does not exists. Create pool first.
		PoolNotFoundError,
		/// The number provided in the arithmetics overflow the type bound. Use lower number.
		ArithmeticsOverflow,
		/// Provided amounts for liquidity are insufficient.
		InsufficientLiquidityProvided,
		/// An error occurred while trying to derive the pool account
		PoolAccountError,
		/// Sender has insufficient balance for this action
		InsufficientAccountBalance,
	}

	impl<T: Config> Pallet<T> {
		/// This function takes two asset identifiers and returns them in a consistent order.
		/// ensures commutativity:  f(a, b) == f(b, a)
		pub fn create_pool_id_from_assets(
			asset_1: AssetIdOf<T>,
			asset_2: AssetIdOf<T>,
		) -> PoolCompositeIdOf<T> {
			if asset_1.encode() < asset_2.encode() {
				(asset_1, asset_2)
			} else {
				(asset_2, asset_1)
			}
		}

		/// Retrieves a pool based on its ID.
		pub fn get_pool_by_id(pool_id: &PoolCompositeIdOf<T>) -> Option<LiquidityPool<T>> {
			Pools::<T>::get(pool_id)
		}

		pub fn pool_exists(pool_id: &PoolCompositeIdOf<T>) -> bool {
			Pools::<T>::contains_key(pool_id)
		}

		pub fn create_liquidity_token_id_for_pair(
			asset_1: AssetIdOf<T>,
			asset_2: AssetIdOf<T>,
		) -> LpAssetId<T> {
			Self::create_liquidity_token_id_for_pool_id(&Self::create_pool_id_from_assets(
				asset_1, asset_2,
			))
		}

		pub fn create_liquidity_token_id_for_pool_id(
			pool_id: &PoolCompositeIdOf<T>,
		) -> LpAssetId<T> {
			let blake = Hashable::blake2_256(&Encode::encode(pool_id));
			Decode::decode(&mut TrailingZeroInput::new(blake.as_ref()))
				.expect("Failed to decode lp token id")
		}

		pub fn derive_pool_account_from_id(
			pool_id: &PoolCompositeIdOf<T>,
		) -> Result<T::AccountId, &'static str> {
			let seed = Hashable::blake2_256(&pool_id.encode());
			T::AccountId::decode(&mut &seed[..]).map_err(|_| "Failed to decode AccountId from seed")
		}

		pub fn ensure_distinct_assets(
			asset_a: &AssetIdOf<T>,
			asset_b: &AssetIdOf<T>,
		) -> Result<(), DispatchError> {
			ensure!(asset_a != asset_b, Error::<T>::DistinctAssetsRequired);
			Ok(())
		}

		pub fn calculate_lp_token_amount_for_pair_amounts(
			amount_a: AssetBalanceOf<T>,
			amount_b: AssetBalanceOf<T>,
		) -> Result<AssetBalanceOf<T>, DispatchError> {
			Ok(amount_a
				.checked_mul(&amount_b)
				.ok_or(Error::<T>::ArithmeticsOverflow)?
				.integer_sqrt())
		}

		pub fn provide_liquidity_to_pool(
			pool_id: PoolCompositeIdOf<T>,
			pool: LiquidityPool<T>,
			amount_a: AssetBalanceOf<T>,
			amount_b: AssetBalanceOf<T>,
		) -> DispatchResult {
			ensure!(
				amount_a > Zero::zero() && amount_b > Zero::zero(),
				Error::<T>::InsufficientLiquidityProvided
			);
			todo!()
		}

		pub fn mint_asset(
			origin: OriginFor<T>,
			asset_id: AssetIdOf<T>,
			receiver: T::AccountId,
			amount: AssetBalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			T::Fungibles::mint_into(asset_id, &receiver, amount)?;
			Ok(())
		}

		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}

		pub fn create_token_if_not_exists(asset_id: AssetIdOf<T>) -> DispatchResult {
			if !T::Fungibles::asset_exists(asset_id.clone()) {
				T::Fungibles::create(asset_id.clone(), Self::account_id(), true, One::one())?;
			}
			Ok(())
		}

		pub fn ensure_amounts_non_zero(
			amount_a: &AssetBalanceOf<T>,
			amount_b: &AssetBalanceOf<T>,
		) -> DispatchResult {
			ensure!(
				*amount_a > Zero::zero() && *amount_b > Zero::zero(),
				Error::<T>::InsufficientLiquidityProvided
			);
			Ok(())
		}

		pub fn get_balance(who: &T::AccountId, asset_id: AssetIdOf<T>) -> AssetBalanceOf<T>{
			T::Fungibles::balance(asset_id, who)
		}

		pub fn ensure_sufficient_balance(
			who: &T::AccountId,
			asset_id: AssetIdOf<T>,
			amount_a: AssetBalanceOf<T>,
		) -> DispatchResult {
			let sender_balance = Self::get_balance(who, asset_id);
			ensure!(sender_balance >= amount_a, Error::<T>::InsufficientAccountBalance);
			Ok(())
		}

		pub fn transfer_assets_and_mint_lp_tokens(
			who: &T::AccountId,
			asset_id_a: AssetIdOf<T>,
			asset_id_b: AssetIdOf<T>,
			amount_a: AssetBalanceOf<T>,
			amount_b: AssetBalanceOf<T>,
			pool_account: T::AccountId,
			liquidity_token_id: LpAssetId<T>,
			liquidity_token_amount: AssetBalanceOf<T>,
		) -> DispatchResult {
			T::Fungibles::transfer(
				asset_id_a.clone(),
				&who.clone(),
				&pool_account.clone(),
				amount_a,
				Preservation::Expendable,
			)?;
			T::Fungibles::transfer(
				asset_id_b.clone(),
				&who,
				&pool_account,
				amount_b,
				Preservation::Expendable,
			)?;
			T::Fungibles::mint_into(
				liquidity_token_id.clone(),
				&who,
				liquidity_token_amount.clone(),
			)?;
			Ok(())
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::default())]
		pub fn initialise_pool_with_assets(
			origin: OriginFor<T>,
			asset_id_a: AssetIdOf<T>,
			asset_id_b: AssetIdOf<T>,
			amount_a: AssetBalanceOf<T>,
			amount_b: AssetBalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::ensure_distinct_assets(&asset_id_a, &asset_id_b)?;
			Self::ensure_amounts_non_zero(&amount_a, &amount_b)?;
			Self::ensure_sufficient_balance(&who, asset_id_a.clone(), amount_a.clone())?;
			Self::ensure_sufficient_balance(&who, asset_id_b.clone(), amount_b.clone())?;
			let pool_id = Self::create_pool_id_from_assets(asset_id_a.clone(), asset_id_b.clone());
			ensure!(!Self::pool_exists(&pool_id), Error::<T>::DuplicatePoolError);

			let liquidity_token_id = Self::create_liquidity_token_id_for_pool_id(&pool_id);
			let lp_token_amount_to_mint: AssetBalanceOf<T> =
				Self::calculate_lp_token_amount_for_pair_amounts(amount_a, amount_b)?;
			Self::create_token_if_not_exists(liquidity_token_id.clone())?;
			let pool = LiquidityPool {
				asset_ids: (asset_id_a.clone(), asset_id_b.clone()),
				balances: (amount_a, amount_b),
				liquidity_token_id: liquidity_token_id.clone(),
			};

			let pool_account = Self::derive_pool_account_from_id(&pool_id)?;

			Pools::<T>::insert(pool_id.clone(), pool);

			Self::deposit_event(Event::PoolCreated {
				pool_id,
				asset_id_a: asset_id_a.clone(),
				asset_id_b: asset_id_b.clone(),
				creator: who.clone(),
				liquidity_token_id: liquidity_token_id.clone(),
				//timestamp_or_block_number: <frame_system::Module<T>>::block_number(),
			});

			Self::transfer_assets_and_mint_lp_tokens(
				&who,
				asset_id_a.clone(),
				asset_id_b.clone(),
				amount_a,
				amount_b,
				pool_account,
				liquidity_token_id,
				lp_token_amount_to_mint,
			)?;

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(Weight::default())]
		pub fn add_liquidity(
			origin: OriginFor<T>,
			asset_id_a: AssetIdOf<T>,
			asset_id_b: AssetIdOf<T>,
			amount_a: AssetBalanceOf<T>,
			amount_b: AssetBalanceOf<T>,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			Self::ensure_distinct_assets(&asset_id_a, &asset_id_b)?;
			let pool_id = Self::create_pool_id_from_assets(asset_id_a.clone(), asset_id_b.clone());
			let pool = Self::get_pool_by_id(&pool_id).ok_or(Error::<T>::PoolNotFoundError)?;

			let liquidity_token_id = Self::create_liquidity_token_id_for_pool_id(&pool_id);
			let lp_token_amount_to_mint: AssetBalanceOf<T> =
				Self::calculate_lp_token_amount_for_pair_amounts(amount_a, amount_b)?;

			Ok(())
		}
	}
}
