// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # Asset Conversion Transaction Payment Pallet
//!
//! This pallet allows runtimes that include it to pay for transactions in assets other than the
//! chain's native asset.
//!
//! ## Overview
//!
//! This pallet provides a `TransactionExtension` with an optional `AssetId` that specifies the
//! asset to be used for payment (defaulting to the native token on `None`). It expects an
//! [`OnChargeAssetTransaction`] implementation analogous to [`pallet-transaction-payment`]. The
//! included [`AssetConversionAdapter`] (implementing [`OnChargeAssetTransaction`]) determines the
//! fee amount by converting the fee calculated by [`pallet-transaction-payment`] in the native
//! asset into the amount required of the specified asset.
//!
//! ## Pallet API
//!
//! This pallet does not have any dispatchable calls or storage. It wraps FRAME's Transaction
//! Payment pallet and functions as a replacement. This means you should include both pallets in
//! your `construct_runtime` macro, but only include this pallet's [`TransactionExtension`]
//! ([`ChargeAssetTxPayment`]).
//!
//! ## Terminology
//!
//! - Native Asset or Native Currency: The asset that a chain considers native, as in its default
//!   for transaction fee payment, deposits, inflation, etc.
//! - Other assets: Other assets that may exist on chain, for example under the Assets pallet.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use codec::{Decode, Encode};
use frame_support::{
	dispatch::{DispatchInfo, DispatchResult, PostDispatchInfo},
	traits::{
		fungibles::{Balanced, Inspect},
		IsType,
	},
	DefaultNoBound,
};
use pallet_transaction_payment::{ChargeTransactionPayment, OnChargeTransaction};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{
		AccrueWeight, AsSystemOriginSigner, DispatchInfoOf, Dispatchable, PostDispatchInfoOf,
		TransactionExtension, TransactionExtensionBase, ValidateResult, Zero,
	},
	transaction_validity::{InvalidTransaction, TransactionValidityError, ValidTransaction},
};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod payment;
use frame_support::{pallet_prelude::Weight, traits::tokens::AssetId};
pub use payment::*;
pub use weights::WeightInfo;

/// Type aliases used for interaction with `OnChargeTransaction`.
pub(crate) type OnChargeTransactionOf<T> =
	<T as pallet_transaction_payment::Config>::OnChargeTransaction;
/// Balance type alias for balances of the chain's native asset.
pub(crate) type BalanceOf<T> = <OnChargeTransactionOf<T> as OnChargeTransaction<T>>::Balance;
/// Liquidity info type alias.
pub(crate) type LiquidityInfoOf<T> =
	<OnChargeTransactionOf<T> as OnChargeTransaction<T>>::LiquidityInfo;

/// Balance type alias for balances of assets that implement the `fungibles` trait.
pub(crate) type AssetBalanceOf<T> =
	<<T as Config>::Fungibles as Inspect<<T as frame_system::Config>::AccountId>>::Balance;
/// Type alias for Asset IDs.
pub(crate) type AssetIdOf<T> =
	<<T as Config>::Fungibles as Inspect<<T as frame_system::Config>::AccountId>>::AssetId;

/// Type alias for the interaction of balances with `OnChargeAssetTransaction`.
pub(crate) type ChargeAssetBalanceOf<T> =
	<<T as Config>::OnChargeAssetTransaction as OnChargeAssetTransaction<T>>::Balance;
/// Type alias for Asset IDs in their interaction with `OnChargeAssetTransaction`.
pub(crate) type ChargeAssetIdOf<T> =
	<<T as Config>::OnChargeAssetTransaction as OnChargeAssetTransaction<T>>::AssetId;
/// Liquidity info type alias for interaction with `OnChargeAssetTransaction`.
pub(crate) type ChargeAssetLiquidityOf<T> =
	<<T as Config>::OnChargeAssetTransaction as OnChargeAssetTransaction<T>>::LiquidityInfo;

/// Used to pass the initial payment info from pre- to post-dispatch.
#[derive(Encode, Decode, DefaultNoBound, TypeInfo)]
pub enum InitialPayment<T: Config> {
	/// No initial fee was paid.
	#[default]
	Nothing,
	/// The initial fee was paid in the native currency.
	Native(LiquidityInfoOf<T>),
	/// The initial fee was paid in an asset.
	Asset((LiquidityInfoOf<T>, BalanceOf<T>, AssetBalanceOf<T>)),
}

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_transaction_payment::Config + pallet_asset_conversion::Config
	{
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The fungibles instance used to pay for transactions in assets.
		type Fungibles: Balanced<Self::AccountId>;
		/// The actual transaction charging logic that charges the fees.
		type OnChargeAssetTransaction: OnChargeAssetTransaction<Self>;
		/// The weight information of this pallet.
		type WeightInfo: WeightInfo;
		#[cfg(feature = "runtime-benchmarks")]
		/// Benchmark helper
		type BenchmarkHelper: BenchmarkHelperTrait<
			Self::AccountId,
			<<Self as Config>::Fungibles as Inspect<Self::AccountId>>::AssetId,
			<<Self as Config>::OnChargeAssetTransaction as OnChargeAssetTransaction<Self>>::AssetId,
		>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[cfg(feature = "runtime-benchmarks")]
	/// Helper trait to benchmark the `ChargeAssetTxPayment` transaction extension.
	pub trait BenchmarkHelperTrait<AccountId, FunAssetIdParameter, AssetIdParameter> {
		/// Returns the `AssetId` to be used in the liquidity pool by the benchmarking code.
		fn create_asset_id_parameter(id: u32) -> (FunAssetIdParameter, AssetIdParameter);
		/// Create a liquidity pool for a given asset and sufficiently endow accounts to benchmark
		/// the extension.
		fn setup_balances_and_pool(asset_id: FunAssetIdParameter, account: AccountId);
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A transaction fee `actual_fee`, of which `tip` was added to the minimum inclusion fee,
		/// has been paid by `who` in an asset `asset_id`.
		AssetTxFeePaid {
			who: T::AccountId,
			actual_fee: AssetBalanceOf<T>,
			tip: BalanceOf<T>,
			asset_id: ChargeAssetIdOf<T>,
		},
		/// A swap of the refund in native currency back to asset failed.
		AssetRefundFailed { native_amount_kept: BalanceOf<T> },
	}
}

/// Require payment for transaction inclusion and optionally include a tip to gain additional
/// priority in the queue. Allows paying via both `Currency` as well as `fungibles::Balanced`.
///
/// Wraps the transaction logic in [`pallet_transaction_payment`] and extends it with assets.
/// An asset ID of `None` falls back to the underlying transaction payment logic via the native
/// currency.
#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ChargeAssetTxPayment<T: Config> {
	#[codec(compact)]
	tip: BalanceOf<T>,
	asset_id: Option<ChargeAssetIdOf<T>>,
}

impl<T: Config> ChargeAssetTxPayment<T>
where
	T::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
	AssetBalanceOf<T>: Send + Sync,
	BalanceOf<T>: Send + Sync + Into<ChargeAssetBalanceOf<T>> + From<ChargeAssetLiquidityOf<T>>,
	ChargeAssetIdOf<T>: Send + Sync,
{
	/// Utility constructor. Used only in client/factory code.
	pub fn from(tip: BalanceOf<T>, asset_id: Option<ChargeAssetIdOf<T>>) -> Self {
		Self { tip, asset_id }
	}

	/// Fee withdrawal logic that dispatches to either `OnChargeAssetTransaction` or
	/// `OnChargeTransaction`.
	fn withdraw_fee(
		&self,
		who: &T::AccountId,
		call: &T::RuntimeCall,
		info: &DispatchInfoOf<T::RuntimeCall>,
		fee: BalanceOf<T>,
	) -> Result<(BalanceOf<T>, InitialPayment<T>), TransactionValidityError> {
		debug_assert!(self.tip <= fee, "tip should be included in the computed fee");
		if fee.is_zero() {
			Ok((fee, InitialPayment::Nothing))
		} else if let Some(asset_id) = &self.asset_id {
			T::OnChargeAssetTransaction::withdraw_fee(
				who,
				call,
				info,
				asset_id.clone(),
				fee.into(),
				self.tip.into(),
			)
			.map(|(used_for_fee, received_exchanged, asset_consumed)| {
				(
					fee,
					InitialPayment::Asset((
						used_for_fee.into(),
						received_exchanged.into(),
						asset_consumed.into(),
					)),
				)
			})
		} else {
			<OnChargeTransactionOf<T> as OnChargeTransaction<T>>::withdraw_fee(
				who, call, info, fee, self.tip,
			)
			.map(|i| (fee, InitialPayment::Native(i)))
			.map_err(|_| -> TransactionValidityError { InvalidTransaction::Payment.into() })
		}
	}

	/// Fee withdrawal logic dry-run that dispatches to either `OnChargeAssetTransaction` or
	/// `OnChargeTransaction`.
	fn can_withdraw_fee(
		&self,
		who: &T::AccountId,
		call: &T::RuntimeCall,
		info: &DispatchInfoOf<T::RuntimeCall>,
		fee: BalanceOf<T>,
	) -> Result<(), TransactionValidityError> {
		debug_assert!(self.tip <= fee, "tip should be included in the computed fee");
		if fee.is_zero() {
			Ok(())
		} else if let Some(asset_id) = &self.asset_id {
			T::OnChargeAssetTransaction::can_withdraw_fee(who, asset_id.clone(), fee.into())
		} else {
			<OnChargeTransactionOf<T> as OnChargeTransaction<T>>::can_withdraw_fee(
				who, call, info, fee, self.tip,
			)
			.map_err(|_| -> TransactionValidityError { InvalidTransaction::Payment.into() })
		}
	}
}

impl<T: Config> core::fmt::Debug for ChargeAssetTxPayment<T> {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		write!(f, "ChargeAssetTxPayment<{:?}, {:?}>", self.tip, self.asset_id.encode())
	}
	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut core::fmt::Formatter) -> core::fmt::Result {
		Ok(())
	}
}

impl<T: Config> TransactionExtensionBase for ChargeAssetTxPayment<T>
where
	AssetBalanceOf<T>: Send + Sync,
	BalanceOf<T>: Send
		+ Sync
		+ From<u64>
		+ Into<ChargeAssetBalanceOf<T>>
		+ Into<ChargeAssetLiquidityOf<T>>
		+ From<ChargeAssetLiquidityOf<T>>,
	ChargeAssetIdOf<T>: Send + Sync,
{
	const IDENTIFIER: &'static str = "ChargeAssetTxPayment";
	type Implicit = ();

	fn weight() -> Weight {
		<T as Config>::WeightInfo::charge_asset_tx_payment_asset()
	}
}

impl<T: Config, Context> TransactionExtension<T::RuntimeCall, Context> for ChargeAssetTxPayment<T>
where
	T::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
	AssetBalanceOf<T>: Send + Sync,
	BalanceOf<T>: Send
		+ Sync
		+ From<u64>
		+ Into<ChargeAssetBalanceOf<T>>
		+ Into<ChargeAssetLiquidityOf<T>>
		+ From<ChargeAssetLiquidityOf<T>>,
	ChargeAssetIdOf<T>: Send + Sync,
	<T::RuntimeCall as Dispatchable>::RuntimeOrigin: AsSystemOriginSigner<T::AccountId> + Clone,
{
	type Val = (
		// tip
		BalanceOf<T>,
		// who paid the fee
		T::AccountId,
		// transaction fee
		BalanceOf<T>,
	);
	type Pre = (
		// tip
		BalanceOf<T>,
		// who paid the fee
		T::AccountId,
		// imbalance resulting from withdrawing the fee
		InitialPayment<T>,
		// asset_id for the transaction payment
		Option<ChargeAssetIdOf<T>>,
	);

	fn validate(
		&self,
		origin: <T::RuntimeCall as Dispatchable>::RuntimeOrigin,
		call: &T::RuntimeCall,
		info: &DispatchInfoOf<T::RuntimeCall>,
		len: usize,
		_context: &mut Context,
		_self_implicit: Self::Implicit,
		_inherited_implication: &impl Encode,
	) -> ValidateResult<Self::Val, T::RuntimeCall> {
		let who = origin.as_system_origin_signer().ok_or(InvalidTransaction::BadSigner)?;
		// Non-mutating call of `compute_fee` to calculate the fee used in the transaction priority.
		let fee = pallet_transaction_payment::Pallet::<T>::compute_fee(len as u32, info, self.tip);
		self.can_withdraw_fee(&who, call, info, fee)?;
		let priority = ChargeTransactionPayment::<T>::get_priority(info, len, self.tip, fee);
		let validity = ValidTransaction { priority, ..Default::default() };
		let val = (self.tip, who.clone(), fee);
		Ok((validity, val, origin))
	}

	fn prepare(
		self,
		val: Self::Val,
		_origin: &<T::RuntimeCall as Dispatchable>::RuntimeOrigin,
		call: &T::RuntimeCall,
		info: &DispatchInfoOf<T::RuntimeCall>,
		_len: usize,
		_context: &Context,
	) -> Result<Self::Pre, TransactionValidityError> {
		let (tip, who, fee) = val;
		// Mutating call of `withdraw_fee` to actually charge for the transaction.
		let (_fee, initial_payment) = self.withdraw_fee(&who, call, info, fee)?;
		Ok((tip, who, initial_payment, self.asset_id.clone()))
	}

	fn post_dispatch(
		pre: Self::Pre,
		info: &DispatchInfoOf<T::RuntimeCall>,
		post_info: &PostDispatchInfoOf<T::RuntimeCall>,
		len: usize,
		result: &DispatchResult,
		_context: &Context,
	) -> Result<Option<Weight>, TransactionValidityError> {
		let (tip, who, initial_payment, asset_id) = pre;
		match initial_payment {
			InitialPayment::Native(already_withdrawn) => {
				debug_assert!(
					asset_id.is_none(),
					"For that payment type the `asset_id` should be None"
				);
				// Take into account the weight used by this extension before calculating the
				// refund.
				let actual_ext_weight = <T as Config>::WeightInfo::charge_asset_tx_payment_native()
					.saturating_sub(
						pallet_transaction_payment::ChargeTransactionPayment::<T>::weight(),
					);
				let mut actual_post_info = post_info.clone();
				actual_post_info.accrue(actual_ext_weight);
				pallet_transaction_payment::ChargeTransactionPayment::<T>::post_dispatch(
					(tip, who, already_withdrawn),
					info,
					&actual_post_info,
					len,
					result,
					&(),
				)?;
				Ok(Some(<T as Config>::WeightInfo::charge_asset_tx_payment_native()))
			},
			InitialPayment::Asset(already_withdrawn) => {
				debug_assert!(
					asset_id.is_some(),
					"For that payment type the `asset_id` should be set"
				);
				// Take into account the weight used by this extension before calculating the
				// refund.
				let actual_ext_weight = <T as Config>::WeightInfo::charge_asset_tx_payment_asset();
				let mut actual_post_info = post_info.clone();
				actual_post_info.accrue(actual_ext_weight);
				let actual_fee = pallet_transaction_payment::Pallet::<T>::compute_actual_fee(
					len as u32,
					info,
					&actual_post_info,
					tip,
				);

				if let Some(asset_id) = asset_id {
					let (used_for_fee, received_exchanged, asset_consumed) = already_withdrawn;
					let converted_fee = T::OnChargeAssetTransaction::correct_and_deposit_fee(
						&who,
						info,
						&actual_post_info,
						actual_fee.into(),
						tip.into(),
						used_for_fee.into(),
						received_exchanged.into(),
						asset_id.clone(),
						asset_consumed.into(),
					)?;

					Pallet::<T>::deposit_event(Event::<T>::AssetTxFeePaid {
						who,
						actual_fee: converted_fee,
						tip,
						asset_id,
					});
				}
				Ok(Some(<T as Config>::WeightInfo::charge_asset_tx_payment_asset()))
			},
			InitialPayment::Nothing => {
				// `actual_fee` should be zero here for any signed extrinsic. It would be
				// non-zero here in case of unsigned extrinsics as they don't pay fees but
				// `compute_actual_fee` is not aware of them. In both cases it's fine to just
				// move ahead without adjusting the fee, though, so we do nothing.
				debug_assert!(tip.is_zero(), "tip should be zero if initial fee was zero.");
				Ok(Some(<T as Config>::WeightInfo::charge_asset_tx_payment_zero()))
			},
		}
	}
}
