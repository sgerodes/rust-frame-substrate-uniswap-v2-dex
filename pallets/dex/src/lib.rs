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

pub type LpAssetId = [u8; 32];

#[frame_support::pallet]
pub mod pallet {
	use crate::{AssetBalanceOf, AssetIdOf, BalanceOf, LpAssetId, PoolCompositeIdOf};
	use frame_support::{
		pallet_prelude::*,
		traits::{fungible, fungibles},
		Hashable,
	};
	use frame_system::pallet_prelude::*;

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
		balances: (BalanceOf<T>, BalanceOf<T>),
		liquidity_token_id: LpAssetId,
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
			liquidity_token_id: LpAssetId,
			// block_number: T::BlockNumber,
			// initial_balances: (AssetBalanceOf<T>, AssetBalanceOf<T>),
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
		/// Can not create a pool that already exists
		DuplicatePoolError,
		/// Assets in the pool must be distinct
		DistinctAssetsRequired,
		/// Trying to do an operation on a pool that does not exists. Create pool first
		PoolNotFoundError,
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
			Self::get_pool_by_id(pool_id).is_some()
		}

		pub fn create_liquidity_token_id_for_pair(
			asset_1: AssetIdOf<T>,
			asset_2: AssetIdOf<T>,
		) -> LpAssetId {
			Self::create_liquidity_token_id_for_pool_id(&Self::create_pool_id_from_assets(
				asset_1, asset_2,
			))
		}

		pub fn create_liquidity_token_id_for_pool_id(pool_id: &PoolCompositeIdOf<T>) -> LpAssetId {
			Hashable::blake2_256(&Encode::encode(pool_id))
		}

		pub fn ensure_distinct_assets(asset_a: &AssetIdOf<T>, asset_b: &AssetIdOf<T>) -> Result<(), DispatchError> {
			ensure!(asset_a != asset_b, Error::<T>::DistinctAssetsRequired);
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
		pub fn create_pool(
			origin: OriginFor<T>,
			asset_id_a: AssetIdOf<T>,
			asset_id_b: AssetIdOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::ensure_distinct_assets(&asset_id_a, &asset_id_b)?;
			let pool_id = Self::create_pool_id_from_assets(asset_id_a.clone(), asset_id_b.clone());
			ensure!(!Self::pool_exists(&pool_id), Error::<T>::DuplicatePoolError);

			let liquidity_token_id = Self::create_liquidity_token_id_for_pool_id(&pool_id);
			let zero_balance: BalanceOf<T> = Default::default();
			let pool = LiquidityPool {
				asset_ids: (asset_id_a.clone(), asset_id_b.clone()),
				balances: (zero_balance, zero_balance),
				liquidity_token_id,
			};

			Pools::<T>::insert(pool_id.clone(), pool);

			Self::deposit_event(Event::PoolCreated {
				pool_id,
				asset_id_a,
				asset_id_b,
				creator: who,
				liquidity_token_id,
				//timestamp_or_block_number: <frame_system::Module<T>>::block_number(),
			});

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
			let who = ensure_signed(origin)?;
			Self::ensure_distinct_assets(&asset_id_a, &asset_id_b)?;
			let pool_id = Self::create_pool_id_from_assets(asset_id_a.clone(), asset_id_b.clone());
			let pool = Self::get_pool_by_id(&pool_id);
			ensure!(pool.is_some(), Error::<T>::PoolNotFoundError);



			Ok(())
		}
	}
}
