//! Benchmarking setup for pallet-voting
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as Voting;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn free_tx() {
		let caller: T::AccountId = whitelisted_caller();
		#[extrinsic_call]
		free_tx(RawOrigin::Signed(caller), true);
	}

	impl_benchmark_test_suite!(Voting, crate::mock::new_test_ext(), crate::mock::Test);
}
