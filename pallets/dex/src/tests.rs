use crate::{mock::*, Error, Event};
use frame_support::{assert_noop, assert_ok};

#[test]
fn it_works_for_default_value() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		// Dispatch a signed extrinsic.
		assert_ok!(Dex::do_something(RuntimeOrigin::signed(1), 42));
		// Read pallet storage and assert an expected result.
		assert_eq!(Dex::something(), Some(42));
		// Assert that the correct event was deposited
		System::assert_last_event(Event::SomethingStored { something: 42, who: 1 }.into());
	});
}

#[test]
fn correct_error_for_none_value() {
	new_test_ext().execute_with(|| {
		// Ensure the expected error is thrown when no value is present.
		assert_noop!(Dex::cause_error(RuntimeOrigin::signed(1)), Error::<Test>::NoneValue);
	});
}

#[cfg(test)]
mod pool_creation_tests {
	use super::*;
	use crate::{mock::*, Error};
	use frame_support::{assert_noop, assert_ok};

	const ALICE_ID: u64 = 1;
	const BOB_ID: u64 = 2;
	const ASSET_ID_A: u32 = 1;
	const ASSET_ID_B: u32 = 2;

	#[test]
	fn fail_create_pool_with_identical_assets() {
		let alice_origin = RuntimeOrigin::signed(ALICE_ID);
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_noop!(
				Dex::create_pool(alice_origin, ASSET_ID_A, ASSET_ID_A),
				Error::<Test>::DistinctAssetsRequired
			);
		});
	}

	#[test]
	fn create_pool_id_from_assets_orders_asset_ids_consistently() {
		// Test that function works regardless of the order of asset IDs
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_eq!(
				Dex::create_pool_id_from_assets(ASSET_ID_A, ASSET_ID_B),
				Dex::create_pool_id_from_assets(ASSET_ID_B, ASSET_ID_A),
				"The pool id function should order asset IDs correctly."
			);
		});
	}

	#[test]
	fn duplicate_pool_creation_should_fail() {
		let bob_origin = RuntimeOrigin::signed(BOB_ID);
		new_test_ext().execute_with(|| {
			System::set_block_number(1);

			// First pool creation should succeed
			assert_ok!(Dex::create_pool(bob_origin.clone(), ASSET_ID_A, ASSET_ID_B));

			// Second pool creation with the same assets should fail
			assert_noop!(
				Dex::create_pool(bob_origin, ASSET_ID_A, ASSET_ID_B),
				Error::<Test>::DuplicatePoolError
			);
		});
	}

	#[test]
	fn it_works() {
		let bob_origin = RuntimeOrigin::signed(BOB_ID);
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_ok!(Dex::create_pool(bob_origin.clone(), ASSET_ID_A, ASSET_ID_B));
			let pool_id = Dex::create_pool_id_from_assets(ASSET_ID_A, ASSET_ID_B);
			// Assert that the correct event was deposited
			let expected_event = Event::PoolCreated {
				pool_id,
				creator: BOB_ID,
				asset_id_a: ASSET_ID_A,
				asset_id_b: ASSET_ID_B,
			};
			System::assert_last_event(expected_event.into());


			// let mut found = false;
			// for event in System::events() {
			// 	if let RuntimeEvent::Dex(crate::Event::PoolCreated {
			// 		asset_id_a, asset_id_b, creator, ..
			// 	}) = event.event {
			// 		assert_eq!(asset_id_a, ASSET_ID_A);
			// 		assert_eq!(asset_id_b, ASSET_ID_B);
			// 		assert_eq!(creator, BOB_ID);
			// 		found = true;
			// 		break;
			// 	}
			// }
			// assert!(found, "Failed to find PoolCreated event");
		});
	}
}
