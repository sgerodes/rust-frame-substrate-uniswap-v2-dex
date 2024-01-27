#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::fungible;
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
	use frame_support::{
		Hashable,
		pallet_prelude::*,
		PalletId, traits::{
			fungible,
			fungibles::{self, Mutate},
			tokens::Preservation,
		},
	};
	use frame_support::traits::fungibles::Create;
	use frame_support::traits::fungibles::Inspect;
	use frame_support::traits::tokens::Fortitude;
	use frame_support::traits::tokens::Precision;
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::{
		AccountIdConversion, CheckedMul, IntegerSquareRoot, One, TrailingZeroInput, Zero,
	};
	use sp_runtime::traits::CheckedAdd;
	use sp_runtime::traits::CheckedDiv;
	use sp_runtime::traits::CheckedSub;

	use crate::{AssetBalanceOf, AssetIdOf, LpAssetId, PoolCompositeIdOf};

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
		pub fn get_asset_id_a(&self) -> AssetIdOf<T> {
			self.asset_a.asset.clone()
		}

		pub fn get_asset_id_b(&self) -> AssetIdOf<T> {
			self.asset_b.asset.clone()
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
			asset_id_a: AssetIdOf<T>,
			asset_id_b: AssetIdOf<T>,
			amount_a: AssetBalanceOf<T>,
			amount_b: AssetBalanceOf<T>,
			liquidity_token_minted: AssetBalanceOf<T>,
		},
		LiquidityRemoved {
			pool_id: PoolCompositeIdOf<T>,
			asset_id_a: AssetIdOf<T>,
			asset_id_b: AssetIdOf<T>,
			amount_a: AssetBalanceOf<T>,
			amount_b: AssetBalanceOf<T>,
			liquidity_token_burned: AssetBalanceOf<T>,
		},
		AssetsSwapped {
			who: T::AccountId,
			asset_id_in: AssetIdOf<T>,
			asset_id_out: AssetIdOf<T>,
			amount_in: AssetBalanceOf<T>,
			amount_out: AssetBalanceOf<T>,
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
		/// Calculation of asset amount based on liquidity tokens failed.
		InvalidLiquidityCalculation,
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
		/// Invallid swap asset
		InvalidSwapAsset,
	}

	impl<T: Config> Pallet<T> {
		/// compares two assets
		pub fn cmp_assets(asset_a: AssetIdOf<T>, asset_b: AssetIdOf<T>) -> bool {
			asset_a.encode() < asset_b.encode()
		}

		/// This function takes two asset identifiers and returns them in a consistent order.
		/// ensures commutativity:  f(a, b) == f(b, a)
		pub fn derive_pool_id_from_assets(
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
			Self::derive_liquidity_token_id_for_pool_id(&Self::derive_pool_id_from_assets(
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
			ensure!(total_lp_supply > Zero::zero(), Error::<T>::InvalidLiquidityCalculation);

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

		/// A * B = k
		/// (A + a) * (B - b) = k
		/// b = B - (k / (A + a))
		fn calculate_swap_amount(
			pool: &LiquidityPool<T>,
			input_asset_id: AssetIdOf<T>,
			input_user_amount: AssetBalanceOf<T>, // a
		) -> Result<AssetBalanceOf<T>, DispatchError> {
			let k_coefficient = Self::calculate_k_coefficient_for_pool(pool)?;
			let (asset_in_pool_balance, asset_out_pool_balance) =
				if pool.asset_a.asset == input_asset_id {
					(pool.asset_a.amount, pool.asset_b.amount)
				} else if pool.asset_b.asset == input_asset_id {
					(pool.asset_b.amount, pool.asset_a.amount)
				} else {
					return Err(Error::<T>::InvalidSwapAsset.into());
				};
			let s1 = asset_in_pool_balance
				.checked_add(&input_user_amount)
				.ok_or(Error::<T>::ArithmeticsOverflow)?;
			let s2 = k_coefficient.checked_div(&s1).ok_or(Error::<T>::ArithmeticsOverflow)?;
			let output_amount =
				asset_out_pool_balance.checked_sub(&s2).ok_or(Error::<T>::ArithmeticsOverflow)?;

			Ok(output_amount)
		}

		fn update_pool_balances(
			pool: &mut LiquidityPool<T>,
			pool_id: &PoolCompositeIdOf<T>,
			input_asset_id: AssetIdOf<T>,
			input_amount: AssetBalanceOf<T>,
			output_amount: AssetBalanceOf<T>,
		) -> DispatchResult {
			if pool.asset_a.asset == input_asset_id {
				pool.asset_a.amount = pool
					.asset_a
					.amount
					.checked_sub(&input_amount)
					.ok_or(Error::<T>::ArithmeticsOverflow)?;

				pool.asset_b.amount = pool
					.asset_b
					.amount
					.checked_add(&output_amount)
					.ok_or(Error::<T>::ArithmeticsOverflow)?;
			} else {
				pool.asset_b.amount = pool
					.asset_b
					.amount
					.checked_sub(&input_amount)
					.ok_or(Error::<T>::ArithmeticsOverflow)?;

				pool.asset_a.amount = pool
					.asset_a
					.amount
					.checked_add(&output_amount)
					.ok_or(Error::<T>::ArithmeticsOverflow)?;
			}

			Pools::<T>::insert(pool_id, pool);

			Ok(())
		}

		/// The coefficient K is important part of the swapping calculations
		/// A * B = k
		pub fn calculate_k_coefficient_for_pool(
			pool: &LiquidityPool<T>,
		) -> Result<AssetBalanceOf<T>, DispatchError> {
			pool.get_asset_a_balance()
				.checked_mul(&pool.get_asset_b_balance())
				.ok_or(Error::<T>::ArithmeticsOverflow.into())
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {

		/// Initializes a liquidity pool for a pair of assets and mints liquidity tokens to the creator.
		///
		/// This function creates a new liquidity pool for a pair of distinct assets and credits the pool creator with liquidity tokens representing their share in the pool. The initial liquidity is defined by the amounts of the two assets provided.
		///
		/// # Parameters
		/// - `origin`: The transaction origin, must be signed by the user initializing the pool.
		/// - `asset_id_a`: The first asset in the pair.
		/// - `asset_id_b`: The second asset in the pair.
		/// - `amount_a`: The amount of the first asset to initialize the pool.
		/// - `amount_b`: The amount of the second asset to initialize the pool.
		///
		/// # Errors
		/// - Returns `DistinctAssetsRequired` if the two assets are the same.
		/// - Returns `InsufficientLiquidityProvided` if the amounts of either asset are zero.
		/// - Returns `InsufficientAccountBalance` if the user does not have enough balance of either asset.
		/// - Returns `DuplicatePoolError` if a pool for the asset pair already exists.
		///
		/// # Events
		/// - Emits `PoolCreated` event when a new liquidity pool is successfully created.
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
			let pool_id = Self::derive_pool_id_from_assets(asset_id_a.clone(), asset_id_b.clone());
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

		/// Adds liquidity to an existing pool and mints liquidity tokens to the liquidity provider.
		///
		/// This function allows users to contribute additional liquidity to an existing pool. In return, the user receives liquidity tokens proportional to their contribution. These tokens can be later used to withdraw assets from the pool.
		///
		/// # Parameters
		/// - `origin`: The transaction origin, must be signed by the user adding liquidity.
		/// - `asset_id_a`: The first asset in the liquidity pair.
		/// - `asset_id_b`: The second asset in the liquidity pair.
		/// - `amount_a`: The amount of the first asset to add to the pool.
		/// - `amount_b`: The amount of the second asset to add to the pool.
		///
		/// # Errors
		/// - Returns `PoolNotFoundError` if the liquidity pool for the asset pair does not exist.
		/// - Returns `DistinctAssetsRequired` if the input assets are the same.
		/// - Returns `InsufficientLiquidityProvided` if the amounts of either asset are zero.
		/// - Returns `InsufficientAccountBalance` if the user does not have enough balance of either asset.
		/// - Returns `LiquidityTokenNotFound` if the liquidity token for the pool does not exist.
		///
		/// # Events
		/// - Emits `LiquiditySupplied` event when liquidity is successfully added to the pool.
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

			let pool_id = Self::derive_pool_id_from_assets(asset_id_a.clone(), asset_id_b.clone());
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

			Self::deposit_event(Event::LiquiditySupplied {
				pool_id,
				asset_id_a,
				asset_id_b,
				amount_a,
				amount_b,
				liquidity_token_minted: lp_token_amount,
			});

			Ok(())
		}


		/// Removes liquidity from a specified asset pair's pool and returns the underlying assets to the user.
		///
		/// This function allows liquidity providers to withdraw their share of liquidity from the pool.
		/// The amount of liquidity to be removed is specified by `lp_amount_provided`.
		/// The user must specify minimum amounts of assets (`min_amount_a` and `min_amount_b`) to protect against slippage.
		///
		/// # Parameters
		/// - `origin`: The transaction origin, must be signed by the liquidity provider.
		/// - `asset_id_a`: The first asset in the liquidity pair.
		/// - `asset_id_b`: The second asset in the liquidity pair.
		/// - `lp_amount_provided`: The amount of liquidity tokens the user wants to burn in exchange for the pool assets.
		/// - `min_amount_a`: The minimum amount of the first asset the user is willing to receive.
		/// - `min_amount_b`: The minimum amount of the second asset the user is willing to receive.
		///
		/// # Errors
		/// - Returns `PoolNotFoundError` if the specified asset pair's pool does not exist.
		/// - Returns `InsufficientLiquidityTokenBalance` if the user does not have enough liquidity tokens.
		/// - Returns `SlippageLimitExceeded` if the actual amounts receivable are less than the specified minimums.
		///
		/// # Events
		/// - Emits `LiquidityRemoved` event when liquidity is successfully removed.
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

			let pool_id = Self::derive_pool_id_from_assets(asset_id_a.clone(), asset_id_b.clone());
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

			Self::deposit_event(Event::LiquidityRemoved {
				pool_id,
				asset_id_a,
				asset_id_b,
				amount_a,
				amount_b,
				liquidity_token_burned: lp_amount_provided,
			});

			Ok(())
		}

		/// Executes a swap of one asset for another in a specified liquidity pool while maintaining the invariant \( A \times B = k \).
		///
		/// The swap is executed based on the constant product formula: \( (A + a) \times (B - b) = k \), where:
		/// - \( A \) and \( B \) are the initial reserves of the two assets in the pool,
		/// - \( a \) is the input amount,
		/// - \( b \) is the output amount.
		///
		/// The function calculates \( b \) as \( B - (k / (A + a)) \), ensuring the invariant holds.
		///
		/// # Parameters
		/// - `origin`: The transaction origin, must be signed by the user initiating the swap.
		/// - `asset_id_in`: The asset being provided by the user.
		/// - `asset_id_out`: The asset the user wishes to receive.
		/// - `amount_in`: The amount of the input asset being swapped.
		/// - `min_amount_out`: The minimum amount of the output asset the user is willing to accept.
		///
		/// # Errors
		/// - Returns `PoolNotFoundError` if the liquidity pool for the asset pair does not exist.
		/// - Returns `DistinctAssetsRequired` if the input and output assets are the same.
		/// - Returns `InvalidLiquidityCalculation` if the input amount is zero.
		/// - Returns `SlippageLimitExceeded` if the output amount is less than the specified minimum.
		///
		/// # Events
		/// - Emits `AssetsSwapped` event when a swap is successfully executed.
		#[pallet::call_index(3)]
		#[pallet::weight(Weight::default())]
		pub fn swap_assets(
			origin: OriginFor<T>,
			asset_id_in: AssetIdOf<T>,
			asset_id_out: AssetIdOf<T>,
			amount_in: AssetBalanceOf<T>,
			min_amount_out: AssetBalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(asset_id_in != asset_id_out, Error::<T>::DistinctAssetsRequired);
			ensure!(amount_in > Zero::zero(), Error::<T>::InvalidLiquidityCalculation);
			let pool_id =
				Self::derive_pool_id_from_assets(asset_id_in.clone(), asset_id_out.clone());
			let mut pool = Self::get_pool_by_id(&pool_id).ok_or(Error::<T>::PoolNotFoundError)?;

			let amount_out =
				Self::calculate_swap_amount(&pool, asset_id_in.clone(), amount_in.clone())?;
			ensure!(amount_out >= min_amount_out, Error::<T>::SlippageLimitExceeded);

			let pool_account = Self::derive_pool_account_from_id(&pool_id)?;

			T::Fungibles::transfer(
				asset_id_in.clone(),
				&who,
				&pool_account,
				amount_in.clone(),
				Preservation::Expendable,
			)?;
			T::Fungibles::transfer(
				asset_id_out.clone(),
				&pool_account,
				&who,
				amount_out.clone(),
				Preservation::Expendable,
			)?;
			Self::update_pool_balances(
				&mut pool,
				&pool_id,
				asset_id_in.clone(),
				amount_in,
				amount_out,
			)?;

			Self::deposit_event(Event::AssetsSwapped {
				who,
				asset_id_in,
				asset_id_out,
				amount_in,
				amount_out,
			});

			Ok(())
		}
	}
}
