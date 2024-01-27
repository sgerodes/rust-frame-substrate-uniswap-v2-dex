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
	use frame_support::traits::tokens::Fortitude;
	use frame_support::traits::tokens::Precision;
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
	use sp_runtime::traits::CheckedAdd;
	use sp_runtime::traits::CheckedDiv;
	use sp_runtime::traits::CheckedSub;
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
	pub struct AssetWithBalance<T: Config> {
		pub asset: AssetIdOf<T>,
		pub amount: AssetBalanceOf<T>,
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct LiquidityPool<T: Config> {
		asset_a: AssetWithBalance<T>,
		asset_b: AssetWithBalance<T>,
		liquidity_token_id: LpAssetId<T>,
	}

	impl<T: Config> LiquidityPool<T> {
		pub fn get_asset_a_balance(&self) -> AssetBalanceOf<T> {
			self.asset_a.amount
		}

		pub fn get_asset_b_balance(&self) -> AssetBalanceOf<T> {
			self.asset_b.amount
		}
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
		/// The arithmetic operation resulted in an overflow beyond the type's limit. Use a smaller value.
		ArithmeticsOverflow,
		/// The amounts provided for liquidity are below the required threshold for this operation.
		InsufficientLiquidityProvided,
		/// Failed to derive the pool account due to an internal error.
		PoolAccountError,
		/// Sender has insufficient balance for this action
		InsufficientAccountBalance,
		/// Liquidity token does not exists in a context where it should
		LiquidityTokenNotFound,
		/// Insufficient liquidity token in users account
		InsufficientLiquidityTokenBalance,
		/// The actual slippage exceeds the user-defined slippage limit.
		SlippageLimitExceeded,
	}

	impl<T: Config> Pallet<T> {
		/// compares two assets
		pub fn cmp_assets(asset_a: AssetIdOf<T>, asset_b: AssetIdOf<T>) -> bool {
			asset_a.encode() < asset_b.encode()
		}

		/// This function takes two asset identifiers and returns them in a consistent order.
		/// ensures commutativity:  f(a, b) == f(b, a)
		pub fn create_pool_id_from_assets(
			asset_a: AssetIdOf<T>,
			asset_b: AssetIdOf<T>,
		) -> PoolCompositeIdOf<T> {
			if Self::cmp_assets(asset_a.clone(), asset_b.clone()) {
				(asset_a, asset_b)
			} else {
				(asset_b, asset_a)
			}
		}

		/// This function takes two asset identifiers and their balances and returns them in a consistent order.
		/// ensures same order as in pool id creation
		pub fn create_assets_with_balances_ordered(
			asset_a: AssetIdOf<T>,
			asset_b: AssetIdOf<T>,
			amount_a: AssetBalanceOf<T>,
			amount_b: AssetBalanceOf<T>,
		) -> (AssetWithBalance<T>, AssetWithBalance<T>) {
			let awb1 = AssetWithBalance { asset: asset_a.clone(), amount: amount_a.clone() };
			let awb2 = AssetWithBalance { asset: asset_b.clone(), amount: amount_b.clone() };
			if Self::cmp_assets(asset_a.clone(), asset_b.clone()) {
				(awb1, awb2)
			} else {
				(awb2, awb1)
			}
		}

		/// Retrieves a pool based on its ID.
		pub fn get_pool_by_id(pool_id: &PoolCompositeIdOf<T>) -> Option<LiquidityPool<T>> {
			Pools::<T>::get(pool_id)
		}

		pub fn pool_exists(pool_id: &PoolCompositeIdOf<T>) -> bool {
			Pools::<T>::contains_key(pool_id)
		}

		pub fn derive_liquidity_token_id_for_pair(
			asset_1: AssetIdOf<T>,
			asset_2: AssetIdOf<T>,
		) -> LpAssetId<T> {
			Self::derive_liquidity_token_id_for_pool_id(&Self::create_pool_id_from_assets(
				asset_1, asset_2,
			))
		}

		pub fn derive_liquidity_token_id_for_pool_id(
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

		pub fn calculate_lp_token_amount(
			amount_a: AssetBalanceOf<T>,
			amount_b: AssetBalanceOf<T>,
		) -> Result<AssetBalanceOf<T>, DispatchError> {
			Ok(amount_a
				.checked_mul(&amount_b)
				.ok_or(Error::<T>::ArithmeticsOverflow)?
				.integer_sqrt())
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
			if !Self::asset_exists(asset_id.clone()) {
				T::Fungibles::create(asset_id.clone(), Self::account_id(), true, One::one())?;
			}
			Ok(())
		}

		pub fn asset_exists(asset_id: AssetIdOf<T>) -> bool {
			T::Fungibles::asset_exists(asset_id.clone())
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

		pub fn get_balance(who: &T::AccountId, asset_id: AssetIdOf<T>) -> AssetBalanceOf<T> {
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

		pub fn transfer_assets_from_user_and_mint_lp_tokens(
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

		pub fn transfer_assets_to_user_and_burn_lp_tokens(
			who: &T::AccountId,
			asset_id_a: AssetIdOf<T>,
			asset_id_b: AssetIdOf<T>,
			amount_a: AssetBalanceOf<T>,
			amount_b: AssetBalanceOf<T>,
			pool_account: T::AccountId,
			liquidity_token_id: LpAssetId<T>,
			liquidity_token_amount: AssetBalanceOf<T>,
		) -> DispatchResult {
			T::Fungibles::burn_from(
				liquidity_token_id,
				&who,
				liquidity_token_amount,
				Precision::Exact,
				Fortitude::Force,
			)?;
			T::Fungibles::transfer(
				asset_id_a.clone(),
				&pool_account,
				&who,
				amount_a,
				Preservation::Expendable,
			)?;
			T::Fungibles::transfer(
				asset_id_b.clone(),
				&pool_account,
				&who,
				amount_b,
				Preservation::Expendable,
			)?;
			Ok(())
		}
		fn calculate_asset_amount(
			total_pool_asset_amount: AssetBalanceOf<T>,
			user_lp_amount: AssetBalanceOf<T>,
			total_lp_supply: AssetBalanceOf<T>,
		) -> Result<AssetBalanceOf<T>, DispatchError> {
			// Ensure that total liquidity supply is not zero to avoid division by zero
			ensure!(total_lp_supply > Zero::zero(), Error::<T>::ArithmeticsOverflow);

			// Calculate the amount of asset corresponding to the fraction
			let asset_amount = total_pool_asset_amount
				.checked_mul(&user_lp_amount)
				.ok_or(Error::<T>::ArithmeticsOverflow)?
				.checked_div(&total_lp_supply)
				.ok_or(Error::<T>::ArithmeticsOverflow)?;

			Ok(asset_amount)
		}

		fn increase_pool_assets(
			pool: &mut LiquidityPool<T>,
			pool_id: &PoolCompositeIdOf<T>,
			asset_with_balance_a: AssetWithBalance<T>,
			asset_with_balance_b: AssetWithBalance<T>,
		) -> DispatchResult {
			pool.asset_a.amount = pool
				.asset_a
				.amount
				.checked_add(&asset_with_balance_a.amount)
				.ok_or(Error::<T>::ArithmeticsOverflow)?;
			pool.asset_b.amount = pool
				.asset_b
				.amount
				.checked_add(&asset_with_balance_b.amount)
				.ok_or(Error::<T>::ArithmeticsOverflow)?;

			Pools::<T>::insert(pool_id, pool);
			Ok(())
		}

		fn decrease_pool_assets(
			pool: &mut LiquidityPool<T>,
			pool_id: &PoolCompositeIdOf<T>,
			amount_a: AssetBalanceOf<T>,
			amount_b: AssetBalanceOf<T>,
		) -> DispatchResult {
			pool.asset_a.amount = pool
				.asset_a
				.amount
				.checked_sub(&amount_a)
				.ok_or(Error::<T>::ArithmeticsOverflow)?;
			pool.asset_b.amount = pool
				.asset_b
				.amount
				.checked_sub(&amount_b)
				.ok_or(Error::<T>::ArithmeticsOverflow)?;

			Pools::<T>::insert(pool_id, pool);
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
			let (asset_with_balance_a, asset_with_balance_b) =
				Self::create_assets_with_balances_ordered(
					asset_id_a.clone(),
					asset_id_b.clone(),
					amount_a.clone(),
					amount_b.clone(),
				);
			ensure!(!Self::pool_exists(&pool_id), Error::<T>::DuplicatePoolError);

			let liquidity_token_id = Self::derive_liquidity_token_id_for_pool_id(&pool_id);
			let lp_token_amount = Self::calculate_lp_token_amount(amount_a, amount_b)?;
			Self::create_token_if_not_exists(liquidity_token_id.clone())?;

			let pool = LiquidityPool {
				asset_a: asset_with_balance_a,
				asset_b: asset_with_balance_b,
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

			Self::transfer_assets_from_user_and_mint_lp_tokens(
				&who,
				asset_id_a.clone(),
				asset_id_b.clone(),
				amount_a,
				amount_b,
				pool_account,
				liquidity_token_id,
				lp_token_amount,
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
			let who = ensure_signed(origin)?;
			Self::ensure_distinct_assets(&asset_id_a, &asset_id_b)?;
			Self::ensure_amounts_non_zero(&amount_a, &amount_b)?;
			Self::ensure_sufficient_balance(&who, asset_id_a.clone(), amount_a)?;
			Self::ensure_sufficient_balance(&who, asset_id_b.clone(), amount_b)?;

			let pool_id = Self::create_pool_id_from_assets(asset_id_a.clone(), asset_id_b.clone());
			let mut pool = Self::get_pool_by_id(&pool_id).ok_or(Error::<T>::PoolNotFoundError)?;
			ensure!(
				Self::asset_exists(pool.liquidity_token_id.clone()),
				Error::<T>::LiquidityTokenNotFound
			);
			let (asset_with_balance_a, asset_with_balance_b) =
				Self::create_assets_with_balances_ordered(
					asset_id_a.clone(),
					asset_id_b.clone(),
					amount_a.clone(),
					amount_b.clone(),
				);

			let lp_token_amount = Self::calculate_lp_token_amount(amount_a, amount_b)?;
			let pool_account = Self::derive_pool_account_from_id(&pool_id)?;

			Self::transfer_assets_from_user_and_mint_lp_tokens(
				&who,
				asset_id_a.clone(),
				asset_id_b.clone(),
				amount_a,
				amount_b,
				pool_account.clone(),
				pool.liquidity_token_id.clone(),
				lp_token_amount.clone(),
			)?;

			Self::increase_pool_assets(
				&mut pool,
				&pool_id,
				asset_with_balance_a,
				asset_with_balance_b,
			)?;

			// Self::deposit_event(Event::LiquiditySupplied {
			// 	pool_id,
			// 	pool.liquidity_token_id.clone(),
			// 	asset_id_a,
			// 	asset_id_b,
			// 	amount_a,
			// 	amount_b,
			// 	liquidity_token_minted: lp_token_amount_to_mint,
			// });

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(Weight::default())]
		pub fn remove_liquidity(
			origin: OriginFor<T>,
			asset_id_a: AssetIdOf<T>,
			asset_id_b: AssetIdOf<T>,
			lp_amount_provided: AssetBalanceOf<T>,
			min_amount_a: AssetBalanceOf<T>,
			min_amount_b: AssetBalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let pool_id = Self::create_pool_id_from_assets(asset_id_a.clone(), asset_id_b.clone());
			ensure!(Self::pool_exists(&pool_id), Error::<T>::PoolNotFoundError);

			let mut pool = Self::get_pool_by_id(&pool_id).ok_or(Error::<T>::PoolNotFoundError)?;
			let lp_id = pool.liquidity_token_id.clone();
			ensure!(Self::asset_exists(lp_id.clone()), Error::<T>::LiquidityTokenNotFound);

			let user_lp_balance = Self::get_balance(&who, lp_id.clone());
			let total_lp_issuance = T::Fungibles::total_issuance(lp_id.clone());
			ensure!(
				user_lp_balance >= lp_amount_provided,
				Error::<T>::InsufficientLiquidityTokenBalance
			);

			// Calculate the amount of each asset to return to the user
			// (This should be based on the pool's current state and the amount of LP tokens being burned)
			let amount_a = Self::calculate_asset_amount(
				pool.asset_a.amount,
				lp_amount_provided,
				total_lp_issuance,
			)?;
			let amount_b = Self::calculate_asset_amount(
				pool.asset_b.amount,
				lp_amount_provided,
				total_lp_issuance,
			)?;

			// Check if the amounts meet the user's expectations to avoid slippage
			ensure!(
				amount_a >= min_amount_a && amount_b >= min_amount_b,
				Error::<T>::SlippageLimitExceeded
			);

			Self::decrease_pool_assets(&mut pool, &pool_id, amount_a, amount_b)?;
			let pool_account = Self::derive_pool_account_from_id(&pool_id)?;

			Self::transfer_assets_to_user_and_burn_lp_tokens(
				&who,
				asset_id_a.clone(),
				asset_id_b.clone(),
				amount_a,
				amount_b,
				pool_account,
				lp_id.clone(),
				lp_amount_provided.clone(),
			)?;

			// Self::deposit_event(Event::LiquidityRemoved {
			// 	pool_id,
			// 	asset_id_a,
			// 	asset_id_b,
			// 	amount_a,
			// 	amount_b,
			// 	liquidity_token_burned: liquidity_token_amount,
			// });

			Ok(())
		}
	}
}
