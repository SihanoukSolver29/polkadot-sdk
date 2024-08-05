
//! Autogenerated weights for `assets_common_migrations`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 38.0.0
//! DATE: 2024-08-05, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `Ciscos-MBP.lan`, CPU: `<UNKNOWN>`
//! WASM-EXECUTION: `Compiled`, CHAIN: `None`, DB CACHE: `1024`

// Executed Command:
// frame-omni-bencher
// v1
// benchmark
// pallet
// --runtime
// target/release/wbuild/asset-hub-rococo-runtime/asset_hub_rococo_runtime.compact.compressed.wasm
// --pallet
// assets_common_migrations
// --extrinsic
// *
// --template
// substrate/.maintain/frame-weight-template.hbs
// --output
// cumulus/parachains/runtimes/assets/asset-hub-rococo/src/weights/assets_common_migrations.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for `assets_common_migrations`.
pub trait WeightInfo {
	fn conversion_step() -> Weight;
}

/// Weights for `assets_common_migrations` using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	/// Storage: `ForeignAssets::Asset` (r:2 w:1)
	/// Proof: `ForeignAssets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	fn conversion_step() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `161`
		//  Estimated: `7556`
		// Minimum execution time: 6_000_000 picoseconds.
		Weight::from_parts(7_000_000, 7556)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	/// Storage: `ForeignAssets::Asset` (r:2 w:1)
	/// Proof: `ForeignAssets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	fn conversion_step() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `161`
		//  Estimated: `7556`
		// Minimum execution time: 6_000_000 picoseconds.
		Weight::from_parts(7_000_000, 7556)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
}
