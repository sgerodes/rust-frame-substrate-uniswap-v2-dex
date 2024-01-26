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

#[frame_support::pallet]
pub mod pallet {
	use crate::{AssetBalanceOf, AssetIdOf, PoolCompositeIdOf};
	use frame_support::{
		pallet_prelude::*,
		traits::{fungible, fungibles},
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

	#[pallet::storage]
	pub type Pools<T> = StorageMap<_, Blake2_128Concat, PoolCompositeIdOf<T>, bool>;

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

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored { something: u32, who: T::AccountId },
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
		pub fn get_pool_by_id(pool_id: &PoolCompositeIdOf<T>) -> Option<bool> {
			Pools::<T>::get(pool_id)
		}

		pub fn pool_exists(pool_id: &PoolCompositeIdOf<T>) -> bool {
			Self::get_pool_by_id(pool_id).is_some()
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
			asset_1: AssetIdOf<T>,
			asset_2: AssetIdOf<T>,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			// ensure the pool is created with distinct assets
			ensure!(asset_1 != asset_2, Error::<T>::DistinctAssetsRequired);

			let pool_id = Self::create_pool_id_from_assets(asset_1, asset_2);

			// ensure the pool does not exist already
			ensure!(Self::get_pool_by_id(&pool_id).is_none(), Error::<T>::DuplicatePoolError);

			Pools::<T>::insert(pool_id, true);

			Ok(())
		}

		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::call_index(100)]
		#[pallet::weight(Weight::default())]
		pub fn do_something(origin: OriginFor<T>, something: u32) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/main-docs/build/origins/
			let who = ensure_signed(origin)?;

			// Update storage.
			<Something<T>>::put(something);

			// Emit an event.
			Self::deposit_event(Event::SomethingStored { something, who });
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}

		/// An example dispatchable that may throw a custom error.
		#[pallet::call_index(101)]
		#[pallet::weight(Weight::default())]
		pub fn cause_error(origin: OriginFor<T>) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// Read a value from storage.
			match <Something<T>>::get() {
				// Return an error if the value has not been set.
				None => Err(Error::<T>::NoneValue.into()),
				Some(old) => {
					// Increment the value read from storage; will error in the event of overflow.
					let new = old.checked_add(1).ok_or(Error::<T>::StorageOverflow)?;
					// Update the value in storage with the incremented result.
					<Something<T>>::put(new);
					Ok(())
				},
			}
		}
	}
}
