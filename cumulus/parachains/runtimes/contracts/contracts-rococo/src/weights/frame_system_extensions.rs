// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Cumulus.

// Cumulus is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Cumulus is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Cumulus.  If not, see <http://www.gnu.org/licenses/>.

//! Autogenerated weights for `frame_system_extensions`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 32.0.0
//! DATE: 2024-10-31, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `697235d969a1`, CPU: `Intel(R) Xeon(R) CPU @ 2.60GHz`
//! WASM-EXECUTION: `Compiled`, CHAIN: `None`, DB CACHE: 1024

// Executed Command:
// frame-omni-bencher
// v1
// benchmark
// pallet
// --extrinsic=*
// --runtime=target/release/wbuild/contracts-rococo-runtime/contracts_rococo_runtime.wasm
// --pallet=frame_system_extensions
// --header=/__w/polkadot-sdk/polkadot-sdk/cumulus/file_header.txt
// --output=./cumulus/parachains/runtimes/contracts/contracts-rococo/src/weights
// --wasm-execution=compiled
// --steps=50
// --repeat=20
// --heap-pages=4096
// --no-storage-info
// --no-min-squares
// --no-median-slopes
// --genesis-builder-policy=none
// --exclude-pallets=pallet_xcm

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `frame_system_extensions`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> frame_system::ExtensionsWeightInfo for WeightInfo<T> {
	/// Storage: `System::BlockHash` (r:1 w:0)
	/// Proof: `System::BlockHash` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	fn check_genesis() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `3509`
		// Minimum execution time: 2_506_000 picoseconds.
		Weight::from_parts(2_609_000, 0)
			.saturating_add(Weight::from_parts(0, 3509))
			.saturating_add(T::DbWeight::get().reads(1))
	}
	/// Storage: `System::BlockHash` (r:1 w:0)
	/// Proof: `System::BlockHash` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	fn check_mortality_mortal_transaction() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `30`
		//  Estimated: `3509`
		// Minimum execution time: 6_809_000 picoseconds.
		Weight::from_parts(7_113_000, 0)
			.saturating_add(Weight::from_parts(0, 3509))
			.saturating_add(T::DbWeight::get().reads(1))
	}
	/// Storage: `System::BlockHash` (r:1 w:0)
	/// Proof: `System::BlockHash` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	fn check_mortality_immortal_transaction() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `68`
		//  Estimated: `3509`
		// Minimum execution time: 7_875_000 picoseconds.
		Weight::from_parts(8_311_000, 0)
			.saturating_add(Weight::from_parts(0, 3509))
			.saturating_add(T::DbWeight::get().reads(1))
	}
	fn check_non_zero_sender() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 734_000 picoseconds.
		Weight::from_parts(775_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
	}
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn check_nonce() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `52`
		//  Estimated: `3593`
		// Minimum execution time: 7_331_000 picoseconds.
		Weight::from_parts(7_624_000, 0)
			.saturating_add(Weight::from_parts(0, 3593))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	fn check_spec_version() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 548_000 picoseconds.
		Weight::from_parts(573_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
	}
	fn check_tx_version() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 527_000 picoseconds.
		Weight::from_parts(559_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
	}
	/// Storage: `System::AllExtrinsicsLen` (r:1 w:1)
	/// Proof: `System::AllExtrinsicsLen` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `System::BlockWeight` (r:1 w:1)
	/// Proof: `System::BlockWeight` (`max_values`: Some(1), `max_size`: Some(48), added: 543, mode: `MaxEncodedLen`)
	/// Storage: `System::ExtrinsicWeightReclaimed` (r:1 w:1)
	/// Proof: `System::ExtrinsicWeightReclaimed` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	fn check_weight() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `1533`
		// Minimum execution time: 6_594_000 picoseconds.
		Weight::from_parts(6_806_000, 0)
			.saturating_add(Weight::from_parts(0, 1533))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `System::ExtrinsicWeightReclaimed` (r:1 w:1)
	/// Proof: `System::ExtrinsicWeightReclaimed` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	/// Storage: `System::BlockWeight` (r:1 w:1)
	/// Proof: `System::BlockWeight` (`max_values`: Some(1), `max_size`: Some(48), added: 543, mode: `MaxEncodedLen`)
	fn weight_reclaim() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `1533`
		// Minimum execution time: 3_680_000 picoseconds.
		Weight::from_parts(3_899_000, 0)
			.saturating_add(Weight::from_parts(0, 1533))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
}