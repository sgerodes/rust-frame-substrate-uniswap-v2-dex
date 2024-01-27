use crate::{mock::*, Error, Event};
use crate::{Config, Pallet};
use frame_support::traits::fungibles::Mutate;
use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::AccountIdConversion;

use crate::mock::{new_test_ext, Test};

#[cfg(test)]
mod dex_tests {
	use super::*;
	use crate::mock::{new_test_ext, Test};
	use crate::{mock::*, Error};
	use codec::Compact;
	use frame_support::traits::fungibles::Inspect;
	use frame_support::{assert_noop, assert_ok};

	const ALICE_ID: u64 = 2;
	const BOB_ID: u64 = 3;
	const ASSET_ID_A: u32 = 1;
	const ASSET_ID_B: u32 = 2;
	const ASSET_ID_C: u32 = 3;

	fn mint_token(receiver: u64, token: u32, amount: u128) {
		assert_ok!(Dex::mint_asset(RuntimeOrigin::root(), token, receiver, amount));
	}

	fn mint_token_creating(receiver: u64, token: u32, amount: u128) {
		let _ = Dex::create_token_if_not_exists(token);
		mint_token(receiver, token, amount);
	}

	fn set_up_alice_with_100_a_b_coins() {
		mint_token_creating(ALICE_ID, ASSET_ID_A, 100);
		mint_token_creating(ALICE_ID, ASSET_ID_B, 100);
	}

	fn set_up_bob_with_100_a_b_coins() {
		mint_token_creating(BOB_ID, ASSET_ID_A, 100);
		mint_token_creating(BOB_ID, ASSET_ID_B, 100);
	}

	#[test]
	fn fail_create_pool_with_identical_assets() {
		let alice_origin = RuntimeOrigin::signed(ALICE_ID);
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_noop!(
				// cloning to create two different objects that equal in value, and not one same object
				Dex::initialise_pool_with_assets(
					alice_origin,
					ASSET_ID_A.clone(),
					ASSET_ID_A.clone(),
					10,
					10
				),
				Error::<Test>::DistinctAssetsRequired
			);
		});
	}

	#[test]
	fn test_pool_id_consistency_regardless_of_asset_order() {
		// Test that function works regardless of the order of asset IDs
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_eq!(
				Dex::create_pool_id_from_assets(ASSET_ID_A, ASSET_ID_B),
				Dex::create_pool_id_from_assets(ASSET_ID_B, ASSET_ID_A),
				"The pool id function should order asset IDs correctly."
			);
			assert_ne!(
				Dex::create_pool_id_from_assets(ASSET_ID_A, ASSET_ID_B),
				Dex::create_pool_id_from_assets(ASSET_ID_B, ASSET_ID_C),
				"The pool id function return different ids for different pairs"
			);
		});
	}

	#[test]
	fn test_lp_token_id_consistency_and_uniqueness_for_asset_pairs() {
		// Test that function works regardless of the order of asset IDs
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_eq!(
				Dex::derive_liquidity_token_id_for_pair(ASSET_ID_A, ASSET_ID_B),
				Dex::derive_liquidity_token_id_for_pair(ASSET_ID_B, ASSET_ID_A),
				"Lp id should be the same for a reverted pair"
			);
			assert_ne!(
				Dex::derive_liquidity_token_id_for_pair(ASSET_ID_A, ASSET_ID_B),
				Dex::derive_liquidity_token_id_for_pair(ASSET_ID_B, ASSET_ID_C),
				"Lp id should be different for different pairs"
			);
		});
	}

	#[test]
	fn duplicate_pool_creation_should_fail() {
		let bob_origin = RuntimeOrigin::signed(BOB_ID);
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			set_up_bob_with_100_a_b_coins();
			set_up_alice_with_100_a_b_coins();
			//let lp_id = Dex::create_liquidity_token_id_for_pair();

			// First pool creation should succeed
			assert_ok!(Dex::initialise_pool_with_assets(
				bob_origin.clone(),
				ASSET_ID_A,
				ASSET_ID_B,
				10,
				10
			));

			// Second pool creation with the same assets should fail
			assert_noop!(
				Dex::initialise_pool_with_assets(bob_origin, ASSET_ID_A, ASSET_ID_B, 10, 10),
				Error::<Test>::DuplicatePoolError
			);
		});
	}

	#[test]
	fn creation_pool_succeeds() {
		let bob_origin = RuntimeOrigin::signed(BOB_ID);
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			set_up_bob_with_100_a_b_coins();

			let pool_id = Dex::create_pool_id_from_assets(ASSET_ID_A, ASSET_ID_B);
			assert!(Dex::get_pool_by_id(&pool_id).is_none());
			assert_ok!(Dex::initialise_pool_with_assets(
				bob_origin.clone(),
				ASSET_ID_A,
				ASSET_ID_B,
				10,
				10
			));
			assert!(Dex::get_pool_by_id(&pool_id).is_some());
		});
	}

	#[test]
	fn pool_creation_emits_correct_event() {
		let bob_origin = RuntimeOrigin::signed(BOB_ID);
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			set_up_bob_with_100_a_b_coins();

			assert_ok!(Dex::initialise_pool_with_assets(
				bob_origin.clone(),
				ASSET_ID_A,
				ASSET_ID_B,
				10,
				10
			));
			let pool_id = Dex::create_pool_id_from_assets(ASSET_ID_A, ASSET_ID_B);
			let lp_id = Dex::derive_liquidity_token_id_for_pair(ASSET_ID_A, ASSET_ID_B);
			// Assert that the correct event was deposited
			// let expected_event = Event::PoolCreated {
			// 	pool_id,
			// 	creator: BOB_ID,
			// 	asset_id_a: ASSET_ID_A,
			// 	asset_id_b: ASSET_ID_B,
			// };
			// System::assert_last_event(expected_event.into());

			let mut found = false;
			for event in System::events() {
				if let RuntimeEvent::Dex(crate::Event::PoolCreated {
					asset_id_a,
					asset_id_b,
					creator,
					liquidity_token_id,
					..
				}) = event.event
				{
					assert_eq!(asset_id_a, ASSET_ID_A);
					assert_eq!(asset_id_b, ASSET_ID_B);
					assert_eq!(creator, BOB_ID);
					assert_eq!(liquidity_token_id, lp_id);
					found = true;
					break;
				}
			}
			assert!(found, "Failed to find PoolCreated event");
		});
	}

	#[test]
	fn calculate_lp_token_amount_for_pair_amounts_overflow() {
		assert!(Dex::calculate_lp_token_amount(u128::MAX, 2).is_err());
	}

	#[test]
	fn calculate_lp_token_amount_for_pair_amounts_works() {
		assert_eq!(Dex::calculate_lp_token_amount(100, 200).unwrap(), 141);
		assert_eq!(Dex::calculate_lp_token_amount(100, 100).unwrap(), 100);
		assert_eq!(Dex::calculate_lp_token_amount(100, 99).unwrap(), 99);
	}

	#[test]
	fn ensure_sufficient_balance_fails_for_low_balance() {
		new_test_ext().execute_with(|| {
			assert_noop!(
				Pallet::<Test>::ensure_sufficient_balance(&ALICE_ID, ASSET_ID_A, 150,),
				Error::<Test>::InsufficientAccountBalance
			);
		});
	}
	#[test]
	fn initialise_pool_fails_due_to_insufficient_balance() {
		let bob_origin = RuntimeOrigin::signed(BOB_ID);
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_noop!(
				Dex::initialise_pool_with_assets(
					bob_origin.clone(),
					ASSET_ID_A,
					ASSET_ID_B,
					10,
					10
				),
				Error::<Test>::InsufficientAccountBalance
			);
		});
	}

	#[test]
	fn mint_asset_increases_balance() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);

			// Assure recipient's initial balance is zero
			let initial_balance = pallet_assets::Pallet::<Test>::balance(ASSET_ID_A, &ALICE_ID);
			assert_eq!(initial_balance, 0, "Initial balance should be zero");

			set_up_alice_with_100_a_b_coins();

			// Check recipient's new balance
			let new_balance = pallet_assets::Pallet::<Test>::balance(ASSET_ID_A, &ALICE_ID);
			assert_eq!(new_balance, 100, "Balance should be equal to the minted amount");
		});
	}

	#[test]
	fn token_creation_is_idempotent() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			let _ = Dex::create_token_if_not_exists(ASSET_ID_A);
			let _ = Dex::create_token_if_not_exists(ASSET_ID_A);
			let _ = Dex::create_token_if_not_exists(ASSET_ID_A);
			let _ = Dex::create_token_if_not_exists(ASSET_ID_A);
		});
	}

	#[test]
	fn lp_tokens_are_correctly_transferred_to_sender() {
		new_test_ext().execute_with(|| {
			set_up_alice_with_100_a_b_coins();
			let amount_a: u128 = 100;
			let amount_b: u128 = 100;

			// Initialize pool with assets
			assert_ok!(Dex::initialise_pool_with_assets(
				RuntimeOrigin::signed(ALICE_ID),
				ASSET_ID_A,
				ASSET_ID_B,
				amount_a,
				amount_b
			));

			// Calculate expected LP token amount
			let expected_lp_token_amount =
				Dex::calculate_lp_token_amount(amount_a, amount_b).unwrap();

			// Assert LP token balance for Alice is increased
			let liquidity_token_id =
				Dex::derive_liquidity_token_id_for_pair(ASSET_ID_A, ASSET_ID_B);
			assert_eq!(
				Dex::get_balance(&ALICE_ID, liquidity_token_id),
				expected_lp_token_amount,
				"LP token balance should have increased by the calculated amount"
			);
			assert_eq!(Dex::get_balance(&ALICE_ID, ASSET_ID_A), 0);
			assert_eq!(Dex::get_balance(&ALICE_ID, ASSET_ID_B), 0);
		});
	}
}
