use frame_support::{assert_noop, assert_ok};

use crate::mock::{new_test_ext, Test};
use crate::Pallet;
use crate::{mock::*, Error};

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
fn set_up_alice_with_u128_max_a_b_coins() {
	mint_token_creating(ALICE_ID, ASSET_ID_A, u128::MAX);
	mint_token_creating(ALICE_ID, ASSET_ID_B, u128::MAX);
}

fn set_up_bob_with_100_a_b_coins() {
	mint_token_creating(BOB_ID, ASSET_ID_A, 100);
	mint_token_creating(BOB_ID, ASSET_ID_B, 100);
}

fn alice_initializes_pool_a_b_with_50() {
	assert_ok!(Dex::initialise_pool_with_assets(
		RuntimeOrigin::signed(ALICE_ID),
		ASSET_ID_A,
		ASSET_ID_B,
		50,
		50
	));
}
fn alice_initializes_pool_a_with_25_and_b_with_75() {
	assert_ok!(Dex::initialise_pool_with_assets(
		RuntimeOrigin::signed(ALICE_ID),
		ASSET_ID_A,
		ASSET_ID_B,
		25,
		75
	));
}

#[cfg(test)]
mod dex_internal_functions_tests {
	use super::*;

	#[test]
	fn test_pool_id_consistency_regardless_of_asset_order() {
		// Test that function works regardless of the order of asset IDs
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_eq!(
				Dex::derive_pool_id_from_assets(ASSET_ID_A, ASSET_ID_B),
				Dex::derive_pool_id_from_assets(ASSET_ID_B, ASSET_ID_A),
				"The pool id function should order asset IDs correctly."
			);
			assert_ne!(
				Dex::derive_pool_id_from_assets(ASSET_ID_A, ASSET_ID_B),
				Dex::derive_pool_id_from_assets(ASSET_ID_B, ASSET_ID_C),
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
}

mod pool_creation_tests {
	use super::*;

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

			let pool_id = Dex::derive_pool_id_from_assets(ASSET_ID_A, ASSET_ID_B);
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
			let _pool_id = Dex::derive_pool_id_from_assets(ASSET_ID_A, ASSET_ID_B);
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

mod add_liquidity_tests {
	use super::*;

	#[test]
	fn successful_liquidity_addition() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);

			set_up_alice_with_100_a_b_coins();
			alice_initializes_pool_a_b_with_50();

			// Verify that Alice's LP token balance has increased
			let liquidity_token_id =
				Dex::derive_liquidity_token_id_for_pair(ASSET_ID_A, ASSET_ID_B);
			let alice_lp_balance_after_init = Dex::get_balance(&ALICE_ID, liquidity_token_id);
			assert!(alice_lp_balance_after_init > 0, "Alice should have received LP tokens");

			assert_ok!(Dex::add_liquidity(
				RuntimeOrigin::signed(ALICE_ID),
				ASSET_ID_A,
				ASSET_ID_B,
				25,
				25
			));

			// Verify that Alice's LP token balance has increased
			let alice_lp_balance_after_add = Dex::get_balance(&ALICE_ID, liquidity_token_id);
			assert!(
				alice_lp_balance_after_add > alice_lp_balance_after_init,
				"Alice should have received LP tokens"
			);

			// Verify that the pool's asset balances have increased
			let pool_id = Dex::derive_pool_id_from_assets(ASSET_ID_A, ASSET_ID_B);
			let pool = Dex::get_pool_by_id(&pool_id).expect("Pool should exist");

			assert_eq!(pool.get_asset_a_balance(), 75, "Asset A balance should be 75");
			assert_eq!(pool.get_asset_b_balance(), 75, "Asset B balance should be 75");
		});
	}

	#[test]
	fn create_pool_with_same_assets_fails() {
		new_test_ext().execute_with(|| {
			let alice_origin = RuntimeOrigin::signed(ALICE_ID);
			assert_noop!(
				Dex::initialise_pool_with_assets(alice_origin, ASSET_ID_A, ASSET_ID_A, 10, 10),
				Error::<Test>::DistinctAssetsRequired
			);
		});
	}

	#[test]
	fn add_liquidity_to_nonexistent_pool_fails() {
		new_test_ext().execute_with(|| {
			let alice_origin = RuntimeOrigin::signed(ALICE_ID);
			set_up_alice_with_100_a_b_coins();
			assert_noop!(
				Dex::add_liquidity(alice_origin, ASSET_ID_A, ASSET_ID_B, 10, 10),
				Error::<Test>::PoolNotFoundError
			);
		});
	}

	#[test]
	fn add_liquidity_with_same_assets_fails() {
		new_test_ext().execute_with(|| {
			set_up_alice_with_100_a_b_coins();
			assert_noop!(
				Dex::add_liquidity(RuntimeOrigin::signed(ALICE_ID), ASSET_ID_A, ASSET_ID_A, 10, 10),
				Error::<Test>::DistinctAssetsRequired
			);
		});
	}

	#[test]
	fn insufficient_balance_for_adding_liquidity_fails() {
		new_test_ext().execute_with(|| {
			set_up_alice_with_100_a_b_coins();
			// Assuming ALICE_ID has only 100 of ASSET_ID_A
			assert_noop!(
				Dex::add_liquidity(
					RuntimeOrigin::signed(ALICE_ID),
					ASSET_ID_A,
					ASSET_ID_B,
					150, // More than Alice's balance
					10
				),
				Error::<Test>::InsufficientAccountBalance
			);
		});
	}
	#[test]
	fn lp_token_amount_overflow_in_add_liquidity_fails() {
		new_test_ext().execute_with(|| {
			set_up_alice_with_u128_max_a_b_coins();
			alice_initializes_pool_a_b_with_50();
			// Use extremely large amounts to potentially cause overflow
			assert_noop!(
				Dex::add_liquidity(
					RuntimeOrigin::signed(ALICE_ID),
					ASSET_ID_A,
					ASSET_ID_B,
					u128::MAX - 50,
					u128::MAX - 50
				),
				Error::<Test>::ArithmeticsOverflow
			);
		});
	}
}

#[cfg(test)]
mod remove_liquidity_tests {
	use super::*;

	#[test]
	fn successful_removal_of_liquidity() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);

			set_up_alice_with_100_a_b_coins();
			alice_initializes_pool_a_b_with_50();

			let liquidity_token_id =
				Dex::derive_liquidity_token_id_for_pair(ASSET_ID_A, ASSET_ID_B);
			let alice_lp_balance_before = Dex::get_balance(&ALICE_ID, liquidity_token_id);
			let alice_asset_a_balance_before = Dex::get_balance(&ALICE_ID, ASSET_ID_A);
			let alice_asset_b_balance_before = Dex::get_balance(&ALICE_ID, ASSET_ID_B);

			assert_ok!(Dex::remove_liquidity(
				RuntimeOrigin::signed(ALICE_ID),
				ASSET_ID_A,
				ASSET_ID_B,
				25,
				5,
				5,
			));

			let alice_lp_balance_after = Dex::get_balance(&ALICE_ID, liquidity_token_id);
			assert!(
				alice_lp_balance_after < alice_lp_balance_before,
				"Alice's LP token balance should have decreased"
			);

			// Verify that Alice's asset balances have increased
			let alice_asset_a_balance = Dex::get_balance(&ALICE_ID, ASSET_ID_A);
			let alice_asset_b_balance = Dex::get_balance(&ALICE_ID, ASSET_ID_B);
			assert!(
				alice_asset_a_balance > alice_asset_a_balance_before
					&& alice_asset_b_balance > alice_asset_b_balance_before,
				"Alice should have received assets back"
			);
		});
	}

	#[test]
	fn successful_removal_of_all_liquidity() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);

			set_up_alice_with_100_a_b_coins();
			alice_initializes_pool_a_b_with_50();

			let liquidity_token_id =
				Dex::derive_liquidity_token_id_for_pair(ASSET_ID_A, ASSET_ID_B);
			let alice_lp_balance_before = Dex::get_balance(&ALICE_ID, liquidity_token_id);
			assert_eq!(alice_lp_balance_before, 50);
			let alice_asset_a_balance_before = Dex::get_balance(&ALICE_ID, ASSET_ID_A);
			let alice_asset_b_balance_before = Dex::get_balance(&ALICE_ID, ASSET_ID_B);
			assert_eq!(alice_asset_a_balance_before, 50);
			assert_eq!(alice_asset_b_balance_before, 50);

			assert_ok!(Dex::remove_liquidity(
				RuntimeOrigin::signed(ALICE_ID),
				ASSET_ID_A,
				ASSET_ID_B,
				50,
				5,
				5,
			));

			let alice_lp_balance_after = Dex::get_balance(&ALICE_ID, liquidity_token_id);
			assert!(
				alice_lp_balance_after < alice_lp_balance_before,
				"Alice's LP token balance should have decreased"
			);
			assert_eq!(alice_lp_balance_after, 0);

			// Verify that Alice's asset balances have increased
			let alice_asset_a_balance = Dex::get_balance(&ALICE_ID, ASSET_ID_A);
			let alice_asset_b_balance = Dex::get_balance(&ALICE_ID, ASSET_ID_B);
			assert!(
				alice_asset_a_balance > alice_asset_a_balance_before
					&& alice_asset_b_balance > alice_asset_b_balance_before,
				"Alice should have received assets back"
			);
			assert_eq!(alice_asset_a_balance, 100);
			assert_eq!(alice_asset_b_balance, 100);
		});
	}

	#[test]
	fn successful_removal_of_liquidity_3() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);

			set_up_alice_with_100_a_b_coins();
			alice_initializes_pool_a_with_25_and_b_with_75();

			let liquidity_token_id =
				Dex::derive_liquidity_token_id_for_pair(ASSET_ID_A, ASSET_ID_B);
			let alice_lp_balance_before = Dex::get_balance(&ALICE_ID, liquidity_token_id);
			let alice_asset_a_balance_before = Dex::get_balance(&ALICE_ID, ASSET_ID_A);
			let alice_asset_b_balance_before = Dex::get_balance(&ALICE_ID, ASSET_ID_B);

			assert_ok!(Dex::remove_liquidity(
				RuntimeOrigin::signed(ALICE_ID),
				ASSET_ID_A,
				ASSET_ID_B,
				25,
				5,
				5,
			));

			let alice_lp_balance_after = Dex::get_balance(&ALICE_ID, liquidity_token_id);
			assert!(
				alice_lp_balance_after < alice_lp_balance_before,
				"Alice's LP token balance should have decreased"
			);

			// Verify that Alice's asset balances have increased
			let alice_asset_a_balance = Dex::get_balance(&ALICE_ID, ASSET_ID_A);
			let alice_asset_b_balance = Dex::get_balance(&ALICE_ID, ASSET_ID_B);
			assert!(
				alice_asset_a_balance > alice_asset_a_balance_before
					&& alice_asset_b_balance > alice_asset_b_balance_before,
				"Alice should have received assets back"
			);
		});
	}

	#[test]
	fn remove_liquidity_from_nonexistent_pool_fails() {
		new_test_ext().execute_with(|| {
			let alice_origin = RuntimeOrigin::signed(ALICE_ID);
			assert_noop!(
				Dex::remove_liquidity(alice_origin, ASSET_ID_A, ASSET_ID_B, 10, 5, 5),
				Error::<Test>::PoolNotFoundError
			);
		});
	}

	#[test]
	fn insufficient_lp_token_balance_fails() {
		new_test_ext().execute_with(|| {
			set_up_alice_with_100_a_b_coins();
			alice_initializes_pool_a_b_with_50();

			// Alice attempts to remove more liquidity than she owns in LP tokens
			assert_noop!(
				Dex::remove_liquidity(
					RuntimeOrigin::signed(ALICE_ID),
					ASSET_ID_A,
					ASSET_ID_B,
					1000,
					10,
					10
				),
				Error::<Test>::InsufficientLiquidityTokenBalance
			);
		});
	}
	#[test]
	fn slippage_limit_exceeded_fails() {
		new_test_ext().execute_with(|| {
			set_up_alice_with_100_a_b_coins();
			alice_initializes_pool_a_b_with_50();

			assert_noop!(
				Dex::remove_liquidity(
					RuntimeOrigin::signed(ALICE_ID),
					ASSET_ID_A,
					ASSET_ID_B,
					25,
					10000, // Unrealistically high minimum amount of Asset A
					1,
				),
				Error::<Test>::SlippageLimitExceeded
			);
			assert_noop!(
				Dex::remove_liquidity(
					RuntimeOrigin::signed(ALICE_ID),
					ASSET_ID_A,
					ASSET_ID_B,
					25,
					1,
					10000, // Unrealistically high minimum amount of Asset B
				),
				Error::<Test>::SlippageLimitExceeded
			);
		});
	}
}
#[cfg(test)]
mod tests {
	use super::*;

	fn alice_swaps_assets(
		asset_id_in: u32,
		asset_id_out: u32,
		amount_in: u128,
		min_amount_out: u128,
	) {
		assert_ok!(Dex::swap_assets(
			RuntimeOrigin::signed(ALICE_ID),
			asset_id_in,
			asset_id_out,
			amount_in,
			min_amount_out,
		));
	}

	#[test]
	fn successful_asset_swap() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);

			set_up_alice_with_100_a_b_coins();
			alice_initializes_pool_a_b_with_50();

			let alice_asset_a_balance_before = Dex::get_balance(&ALICE_ID, ASSET_ID_A);
			let alice_asset_b_balance_before = Dex::get_balance(&ALICE_ID, ASSET_ID_B);

			let swap_amount_in = 10;
			let min_amount_out = 1;
			alice_swaps_assets(ASSET_ID_A, ASSET_ID_B, swap_amount_in, min_amount_out);

			let alice_asset_a_balance_after = Dex::get_balance(&ALICE_ID, ASSET_ID_A);
			let alice_asset_b_balance_after = Dex::get_balance(&ALICE_ID, ASSET_ID_B);
			assert!(
				alice_asset_a_balance_after < alice_asset_a_balance_before,
				"Alice's Asset A balance should have decreased"
			);
			assert!(
				alice_asset_b_balance_after > alice_asset_b_balance_before,
				"Alice's Asset B balance should have increased"
			);

			let pool_id = Dex::derive_pool_id_from_assets(ASSET_ID_A, ASSET_ID_B);
			let pool = Dex::get_pool_by_id(&pool_id).expect("Pool should exist");

			if pool.get_asset_id_a() == ASSET_ID_A {
				assert_eq!(pool.get_asset_a_balance(), 40);
				assert!(
					pool.get_asset_a_balance() < 50 && pool.get_asset_b_balance() > 50,
					"Pool's asset balances should reflect the swap"
				);
			} else {
				assert_eq!(pool.get_asset_a_balance(), 60);
				assert!(
					pool.get_asset_a_balance() > 50 && pool.get_asset_b_balance() < 50,
					"Pool's asset balances should reflect the swap"
				);
			}
		});
	}

	#[test]
	fn successful_swap() {
		new_test_ext().execute_with(|| {
			set_up_alice_with_100_a_b_coins();
			alice_initializes_pool_a_b_with_50();

			assert_ok!(Dex::swap_assets(
				RuntimeOrigin::signed(ALICE_ID),
				ASSET_ID_A,
				ASSET_ID_B,
				10,
				1,
			));
		});
	}
}

#[cfg(test)]
mod price_oracle_tests {
	use super::*;

	#[test]
	fn successful_get_price() {
		new_test_ext().execute_with(|| {
			set_up_alice_with_100_a_b_coins();
			alice_initializes_pool_a_b_with_50();
			let price = Dex::get_price(RuntimeOrigin::signed(ALICE_ID), ASSET_ID_A, ASSET_ID_B);
			assert_eq!(price, Ok(1));
		});
	}

	#[test]
	fn successful_get_price_2() {
		new_test_ext().execute_with(|| {
			set_up_alice_with_100_a_b_coins();
			alice_initializes_pool_a_with_25_and_b_with_75();
			let price = Dex::get_price(RuntimeOrigin::signed(ALICE_ID), ASSET_ID_B, ASSET_ID_A);
			assert_eq!(price, Ok(3));
		});
	}
}
