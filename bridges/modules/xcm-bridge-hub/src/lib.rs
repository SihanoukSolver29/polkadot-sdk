// Copyright 2019-2021 Parity Technologies (UK) Ltd.
// This file is part of Parity Bridges Common.

// Parity Bridges Common is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Bridges Common is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Bridges Common.  If not, see <http://www.gnu.org/licenses/>.

//! Module that adds XCM support to bridge pallets.

#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

<<<<<<< HEAD
use bridge_runtime_common::messages_xcm_extension::XcmBlobHauler;
use pallet_bridge_messages::Config as BridgeMessagesConfig;
=======
use bp_messages::{LaneState, MessageNonce};
use bp_runtime::{AccountIdOf, BalanceOf, RangeInclusiveExt};
pub use bp_xcm_bridge_hub::{Bridge, BridgeId, BridgeState};
use bp_xcm_bridge_hub::{BridgeLocations, BridgeLocationsError, LocalXcmChannelManager};
use frame_support::{traits::fungible::MutateHold, DefaultNoBound};
use frame_system::Config as SystemConfig;
use pallet_bridge_messages::{Config as BridgeMessagesConfig, LanesManagerError};
use sp_runtime::traits::Zero;
use sp_std::{boxed::Box, vec::Vec};
>>>>>>> 710e74d (Bridges lane id agnostic for backwards compatibility (#5649))
use xcm::prelude::*;

pub use exporter::PalletAsHaulBlobExporter;
pub use pallet::*;

mod exporter;
mod mock;

/// The target that will be used when publishing logs related to this pallet.
pub const LOG_TARGET: &str = "runtime::bridge-xcm";

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use bridge_runtime_common::messages_xcm_extension::SenderAndLane;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::BlockNumberFor;

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config<I: 'static = ()>:
		BridgeMessagesConfig<Self::BridgeMessagesPalletInstance>
	{
		/// Runtime's universal location.
		type UniversalLocation: Get<InteriorLocation>;
		// TODO: https://github.com/paritytech/parity-bridges-common/issues/1666 remove `ChainId` and
		// replace it with the `NetworkId` - then we'll be able to use
		// `T as pallet_bridge_messages::Config<T::BridgeMessagesPalletInstance>::BridgedChain::NetworkId`
		/// Bridged network as relative location of bridged `GlobalConsensus`.
		#[pallet::constant]
		type BridgedNetwork: Get<Location>;
		/// Associated messages pallet instance that bridges us with the
		/// `BridgedNetworkId` consensus.
		type BridgeMessagesPalletInstance: 'static;

		/// Price of single message export to the bridged consensus (`Self::BridgedNetworkId`).
		type MessageExportPrice: Get<Assets>;
		/// Checks the XCM version for the destination.
		type DestinationVersion: GetVersion;

<<<<<<< HEAD
		/// Get point-to-point links with bridged consensus (`Self::BridgedNetworkId`).
		/// (this will be replaced with dynamic on-chain bridges - `Bridges V2`)
		type Lanes: Get<sp_std::vec::Vec<(SenderAndLane, (NetworkId, InteriorLocation))>>;
		/// Support for point-to-point links
		/// (this will be replaced with dynamic on-chain bridges - `Bridges V2`)
		type LanesSupport: XcmBlobHauler;
	}

=======
		/// The origin that is allowed to call privileged operations on the pallet, e.g. open/close
		/// bridge for locations.
		type ForceOrigin: EnsureOrigin<<Self as SystemConfig>::RuntimeOrigin>;
		/// A set of XCM locations within local consensus system that are allowed to open
		/// bridges with remote destinations.
		type OpenBridgeOrigin: EnsureOrigin<
			<Self as SystemConfig>::RuntimeOrigin,
			Success = Location,
		>;
		/// A converter between a location and a sovereign account.
		type BridgeOriginAccountIdConverter: ConvertLocation<AccountIdOf<ThisChainOf<Self, I>>>;

		/// Amount of this chain native tokens that is reserved on the sibling parachain account
		/// when bridge open request is registered.
		#[pallet::constant]
		type BridgeDeposit: Get<BalanceOf<ThisChainOf<Self, I>>>;
		/// Currency used to pay for bridge registration.
		type Currency: MutateHold<
			AccountIdOf<ThisChainOf<Self, I>>,
			Balance = BalanceOf<ThisChainOf<Self, I>>,
			Reason = Self::RuntimeHoldReason,
		>;
		/// The overarching runtime hold reason.
		type RuntimeHoldReason: From<HoldReason<I>>;
		/// Do not hold `Self::BridgeDeposit` for the location of `Self::OpenBridgeOrigin`.
		/// For example, it is possible to make an exception for a system parachain or relay.
		type AllowWithoutBridgeDeposit: Contains<Location>;

		/// Local XCM channel manager.
		type LocalXcmChannelManager: LocalXcmChannelManager;
		/// XCM-level dispatcher for inbound bridge messages.
		type BlobDispatcher: DispatchBlob;
	}

	/// An alias for the bridge metadata.
	pub type BridgeOf<T, I> = Bridge<ThisChainOf<T, I>, LaneIdOf<T, I>>;
	/// An alias for this chain.
	pub type ThisChainOf<T, I> =
		pallet_bridge_messages::ThisChainOf<T, <T as Config<I>>::BridgeMessagesPalletInstance>;
	/// An alias for lane identifier type.
	pub type LaneIdOf<T, I> =
		<T as BridgeMessagesConfig<<T as Config<I>>::BridgeMessagesPalletInstance>>::LaneId;
	/// An alias for the associated lanes manager.
	pub type LanesManagerOf<T, I> =
		pallet_bridge_messages::LanesManager<T, <T as Config<I>>::BridgeMessagesPalletInstance>;

>>>>>>> 710e74d (Bridges lane id agnostic for backwards compatibility (#5649))
	#[pallet::pallet]
	pub struct Pallet<T, I = ()>(PhantomData<(T, I)>);

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {
		fn integrity_test() {
			assert!(
				Self::bridged_network_id().is_some(),
				"Configured `T::BridgedNetwork`: {:?} does not contain `GlobalConsensus` junction with `NetworkId`",
				T::BridgedNetwork::get()
			)
		}
<<<<<<< HEAD
	}

	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Returns dedicated/configured lane identifier.
		pub(crate) fn lane_for(
			source: &InteriorLocation,
			dest: (&NetworkId, &InteriorLocation),
		) -> Option<SenderAndLane> {
			let source = source.clone().relative_to(&T::UniversalLocation::get());

			// Check that we have configured a point-to-point lane for 'source' and `dest`.
			T::Lanes::get()
				.into_iter()
				.find_map(|(lane_source, (lane_dest_network, lane_dest))| {
					if lane_source.location == source &&
						&lane_dest_network == dest.0 &&
						Self::bridged_network_id().as_ref() == Some(dest.0) &&
						&lane_dest == dest.1
					{
						Some(lane_source)
					} else {
						None
					}
				})
		}

=======

		#[cfg(feature = "try-runtime")]
		fn try_state(_n: BlockNumberFor<T>) -> Result<(), sp_runtime::TryRuntimeError> {
			Self::do_try_state()
		}
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Open a bridge between two locations.
		///
		/// The caller must be within the `T::OpenBridgeOrigin` filter (presumably: a sibling
		/// parachain or a parent relay chain). The `bridge_destination_universal_location` must be
		/// a destination within the consensus of the `T::BridgedNetwork` network.
		///
		/// The `BridgeDeposit` amount is reserved on the caller account. This deposit
		/// is unreserved after bridge is closed.
		///
		/// The states after this call: bridge is `Opened`, outbound lane is `Opened`, inbound lane
		/// is `Opened`.
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::zero())] // TODO:(bridges-v2) - https://github.com/paritytech/parity-bridges-common/issues/3046 - add benchmarks impl
		pub fn open_bridge(
			origin: OriginFor<T>,
			bridge_destination_universal_location: Box<VersionedInteriorLocation>,
		) -> DispatchResult {
			// check and compute required bridge locations and laneId
			let xcm_version = bridge_destination_universal_location.identify_version();
			let locations =
				Self::bridge_locations_from_origin(origin, bridge_destination_universal_location)?;
			let lane_id = locations.calculate_lane_id(xcm_version).map_err(|e| {
				log::trace!(
					target: LOG_TARGET,
					"calculate_lane_id error: {e:?}",
				);
				Error::<T, I>::BridgeLocations(e)
			})?;

			Self::do_open_bridge(locations, lane_id, true)
		}

		/// Try to close the bridge.
		///
		/// Can only be called by the "owner" of this side of the bridge, meaning that the
		/// inbound XCM channel with the local origin chain is working.
		///
		/// Closed bridge is a bridge without any traces in the runtime storage. So this method
		/// first tries to prune all queued messages at the outbound lane. When there are no
		/// outbound messages left, outbound and inbound lanes are purged. After that, funds
		/// are returned back to the owner of this side of the bridge.
		///
		/// The number of messages that we may prune in a single call is limited by the
		/// `may_prune_messages` argument. If there are more messages in the queue, the method
		/// prunes exactly `may_prune_messages` and exits early. The caller may call it again
		/// until outbound queue is depleted and get his funds back.
		///
		/// The states after this call: everything is either `Closed`, or purged from the
		/// runtime storage.
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::zero())] // TODO:(bridges-v2) - https://github.com/paritytech/parity-bridges-common/issues/3046 - add benchmarks impl
		pub fn close_bridge(
			origin: OriginFor<T>,
			bridge_destination_universal_location: Box<VersionedInteriorLocation>,
			may_prune_messages: MessageNonce,
		) -> DispatchResult {
			// compute required bridge locations
			let locations =
				Self::bridge_locations_from_origin(origin, bridge_destination_universal_location)?;

			// TODO: https://github.com/paritytech/parity-bridges-common/issues/1760 - may do refund here, if
			// bridge/lanes are already closed + for messages that are not pruned

			// update bridge metadata - this also guarantees that the bridge is in the proper state
			let bridge =
				Bridges::<T, I>::try_mutate_exists(locations.bridge_id(), |bridge| match bridge {
					Some(bridge) => {
						bridge.state = BridgeState::Closed;
						Ok(bridge.clone())
					},
					None => Err(Error::<T, I>::UnknownBridge),
				})?;

			// close inbound and outbound lanes
			let lanes_manager = LanesManagerOf::<T, I>::new();
			let mut inbound_lane = lanes_manager
				.any_state_inbound_lane(bridge.lane_id)
				.map_err(Error::<T, I>::LanesManager)?;
			let mut outbound_lane = lanes_manager
				.any_state_outbound_lane(bridge.lane_id)
				.map_err(Error::<T, I>::LanesManager)?;

			// now prune queued messages
			let mut pruned_messages = 0;
			for _ in outbound_lane.queued_messages() {
				if pruned_messages == may_prune_messages {
					break
				}

				outbound_lane.remove_oldest_unpruned_message();
				pruned_messages += 1;
			}

			// if there are outbound messages in the queue, just update states and early exit
			if !outbound_lane.queued_messages().is_empty() {
				// update lanes state. Under normal circumstances, following calls shall never fail
				inbound_lane.set_state(LaneState::Closed);
				outbound_lane.set_state(LaneState::Closed);

				// write something to log
				let enqueued_messages = outbound_lane.queued_messages().saturating_len();
				log::trace!(
					target: LOG_TARGET,
					"Bridge {:?} between {:?} and {:?} is closing lane_id: {:?}. {} messages remaining",
					locations.bridge_id(),
					locations.bridge_origin_universal_location(),
					locations.bridge_destination_universal_location(),
					bridge.lane_id,
					enqueued_messages,
				);

				// deposit the `ClosingBridge` event
				Self::deposit_event(Event::<T, I>::ClosingBridge {
					bridge_id: *locations.bridge_id(),
					lane_id: bridge.lane_id.into(),
					pruned_messages,
					enqueued_messages,
				});

				return Ok(())
			}

			// else we have pruned all messages, so lanes and the bridge itself may gone
			inbound_lane.purge();
			outbound_lane.purge();
			Bridges::<T, I>::remove(locations.bridge_id());
			LaneToBridge::<T, I>::remove(bridge.lane_id);

			// return deposit
			let released_deposit = T::Currency::release(
				&HoldReason::BridgeDeposit.into(),
				&bridge.bridge_owner_account,
				bridge.deposit,
				Precision::BestEffort,
			)
			.map_err(|e| {
				// we can't do anything here - looks like funds have been (partially) unreserved
				// before by someone else. Let's not fail, though - it'll be worse for the caller
				log::error!(
					target: LOG_TARGET,
					"Failed to unreserve during the bridge {:?} closure with error: {e:?}",
					locations.bridge_id(),
				);
				e
			})
			.ok()
			.unwrap_or(BalanceOf::<ThisChainOf<T, I>>::zero());

			// write something to log
			log::trace!(
				target: LOG_TARGET,
				"Bridge {:?} between {:?} and {:?} has closed lane_id: {:?}, the bridge deposit {released_deposit:?} was returned",
				locations.bridge_id(),
				bridge.lane_id,
				locations.bridge_origin_universal_location(),
				locations.bridge_destination_universal_location(),
			);

			// deposit the `BridgePruned` event
			Self::deposit_event(Event::<T, I>::BridgePruned {
				bridge_id: *locations.bridge_id(),
				lane_id: bridge.lane_id.into(),
				bridge_deposit: released_deposit,
				pruned_messages,
			});

			Ok(())
		}
	}

	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Open bridge for lane.
		pub fn do_open_bridge(
			locations: Box<BridgeLocations>,
			lane_id: T::LaneId,
			create_lanes: bool,
		) -> Result<(), DispatchError> {
			// reserve balance on the origin's sovereign account (if needed)
			let bridge_owner_account = T::BridgeOriginAccountIdConverter::convert_location(
				locations.bridge_origin_relative_location(),
			)
			.ok_or(Error::<T, I>::InvalidBridgeOriginAccount)?;
			let deposit = if T::AllowWithoutBridgeDeposit::contains(
				locations.bridge_origin_relative_location(),
			) {
				BalanceOf::<ThisChainOf<T, I>>::zero()
			} else {
				let deposit = T::BridgeDeposit::get();
				T::Currency::hold(
					&HoldReason::BridgeDeposit.into(),
					&bridge_owner_account,
					deposit,
				)
				.map_err(|e| {
					log::error!(
						target: LOG_TARGET,
						"Failed to hold bridge deposit: {deposit:?} \
						from bridge_owner_account: {bridge_owner_account:?} derived from \
						bridge_origin_relative_location: {:?} with error: {e:?}",
						locations.bridge_origin_relative_location(),
					);
					Error::<T, I>::FailedToReserveBridgeDeposit
				})?;
				deposit
			};

			// save bridge metadata
			Bridges::<T, I>::try_mutate(locations.bridge_id(), |bridge| match bridge {
				Some(_) => Err(Error::<T, I>::BridgeAlreadyExists),
				None => {
					*bridge = Some(BridgeOf::<T, I> {
						bridge_origin_relative_location: Box::new(
							locations.bridge_origin_relative_location().clone().into(),
						),
						bridge_origin_universal_location: Box::new(
							locations.bridge_origin_universal_location().clone().into(),
						),
						bridge_destination_universal_location: Box::new(
							locations.bridge_destination_universal_location().clone().into(),
						),
						state: BridgeState::Opened,
						bridge_owner_account,
						deposit,
						lane_id,
					});
					Ok(())
				},
			})?;
			// save lane to bridge mapping
			LaneToBridge::<T, I>::try_mutate(lane_id, |bridge| match bridge {
				Some(_) => Err(Error::<T, I>::BridgeAlreadyExists),
				None => {
					*bridge = Some(*locations.bridge_id());
					Ok(())
				},
			})?;

			if create_lanes {
				// create new lanes. Under normal circumstances, following calls shall never fail
				let lanes_manager = LanesManagerOf::<T, I>::new();
				lanes_manager
					.create_inbound_lane(lane_id)
					.map_err(Error::<T, I>::LanesManager)?;
				lanes_manager
					.create_outbound_lane(lane_id)
					.map_err(Error::<T, I>::LanesManager)?;
			}

			// write something to log
			log::trace!(
				target: LOG_TARGET,
				"Bridge {:?} between {:?} and {:?} has been opened using lane_id: {lane_id:?}",
				locations.bridge_id(),
				locations.bridge_origin_universal_location(),
				locations.bridge_destination_universal_location(),
			);

			// deposit `BridgeOpened` event
			Self::deposit_event(Event::<T, I>::BridgeOpened {
				bridge_id: *locations.bridge_id(),
				bridge_deposit: deposit,
				local_endpoint: Box::new(locations.bridge_origin_universal_location().clone()),
				remote_endpoint: Box::new(
					locations.bridge_destination_universal_location().clone(),
				),
				lane_id: lane_id.into(),
			});

			Ok(())
		}
	}

	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Return bridge endpoint locations and dedicated lane identifier. This method converts
		/// runtime `origin` argument to relative `Location` using the `T::OpenBridgeOrigin`
		/// converter.
		pub fn bridge_locations_from_origin(
			origin: OriginFor<T>,
			bridge_destination_universal_location: Box<VersionedInteriorLocation>,
		) -> Result<Box<BridgeLocations>, sp_runtime::DispatchError> {
			Self::bridge_locations(
				T::OpenBridgeOrigin::ensure_origin(origin)?,
				(*bridge_destination_universal_location)
					.try_into()
					.map_err(|_| Error::<T, I>::UnsupportedXcmVersion)?,
			)
		}

		/// Return bridge endpoint locations and dedicated **bridge** identifier (`BridgeId`).
		pub fn bridge_locations(
			bridge_origin_relative_location: Location,
			bridge_destination_universal_location: InteriorLocation,
		) -> Result<Box<BridgeLocations>, sp_runtime::DispatchError> {
			BridgeLocations::bridge_locations(
				T::UniversalLocation::get(),
				bridge_origin_relative_location,
				bridge_destination_universal_location,
				Self::bridged_network_id()?,
			)
			.map_err(|e| {
				log::trace!(
					target: LOG_TARGET,
					"bridge_locations error: {e:?}",
				);
				Error::<T, I>::BridgeLocations(e).into()
			})
		}

		/// Return bridge metadata by bridge_id
		pub fn bridge(bridge_id: &BridgeId) -> Option<BridgeOf<T, I>> {
			Bridges::<T, I>::get(bridge_id)
		}

		/// Return bridge metadata by lane_id
		pub fn bridge_by_lane_id(lane_id: &T::LaneId) -> Option<(BridgeId, BridgeOf<T, I>)> {
			LaneToBridge::<T, I>::get(lane_id)
				.and_then(|bridge_id| Self::bridge(&bridge_id).map(|bridge| (bridge_id, bridge)))
		}
	}

	impl<T: Config<I>, I: 'static> Pallet<T, I> {
>>>>>>> 710e74d (Bridges lane id agnostic for backwards compatibility (#5649))
		/// Returns some `NetworkId` if contains `GlobalConsensus` junction.
		fn bridged_network_id() -> Option<NetworkId> {
			match T::BridgedNetwork::get().take_first_interior() {
				Some(GlobalConsensus(network)) => Some(network),
				_ => None,
			}
		}
	}
<<<<<<< HEAD
=======

	#[cfg(any(test, feature = "try-runtime", feature = "std"))]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Ensure the correctness of the state of this pallet.
		pub fn do_try_state() -> Result<(), sp_runtime::TryRuntimeError> {
			use sp_std::collections::btree_set::BTreeSet;

			let mut lanes = BTreeSet::new();

			// check all known bridge configurations
			for (bridge_id, bridge) in Bridges::<T, I>::iter() {
				lanes.insert(Self::do_try_state_for_bridge(bridge_id, bridge)?);
			}
			ensure!(
				lanes.len() == Bridges::<T, I>::iter().count(),
				"Invalid `Bridges` configuration, probably two bridges handle the same laneId!"
			);
			ensure!(
				lanes.len() == LaneToBridge::<T, I>::iter().count(),
				"Invalid `LaneToBridge` configuration, probably missing or not removed laneId!"
			);

			// check connected `pallet_bridge_messages` state.
			Self::do_try_state_for_messages()
		}

		/// Ensure the correctness of the state of the bridge.
		pub fn do_try_state_for_bridge(
			bridge_id: BridgeId,
			bridge: BridgeOf<T, I>,
		) -> Result<T::LaneId, sp_runtime::TryRuntimeError> {
			log::info!(target: LOG_TARGET, "Checking `do_try_state_for_bridge` for bridge_id: {bridge_id:?} and bridge: {bridge:?}");

			// check `BridgeId` points to the same `LaneId` and vice versa.
			ensure!(
				Some(bridge_id) == LaneToBridge::<T, I>::get(bridge.lane_id),
				"Found `LaneToBridge` inconsistency for bridge_id - missing mapping!"
			);

			// check `pallet_bridge_messages` state for that `LaneId`.
			let lanes_manager = LanesManagerOf::<T, I>::new();
			ensure!(
				lanes_manager.any_state_inbound_lane(bridge.lane_id).is_ok(),
				"Inbound lane not found!",
			);
			ensure!(
				lanes_manager.any_state_outbound_lane(bridge.lane_id).is_ok(),
				"Outbound lane not found!",
			);

			// check that `locations` are convertible to the `latest` XCM.
			let bridge_origin_relative_location_as_latest: &Location =
				bridge.bridge_origin_relative_location.try_as().map_err(|_| {
					"`bridge.bridge_origin_relative_location` cannot be converted to the `latest` XCM, needs migration!"
				})?;
			let bridge_origin_universal_location_as_latest: &InteriorLocation = bridge.bridge_origin_universal_location
				.try_as()
				.map_err(|_| "`bridge.bridge_origin_universal_location` cannot be converted to the `latest` XCM, needs migration!")?;
			let bridge_destination_universal_location_as_latest: &InteriorLocation = bridge.bridge_destination_universal_location
				.try_as()
				.map_err(|_| "`bridge.bridge_destination_universal_location` cannot be converted to the `latest` XCM, needs migration!")?;

			// check `BridgeId` does not change
			ensure!(
				bridge_id == BridgeId::new(bridge_origin_universal_location_as_latest, bridge_destination_universal_location_as_latest),
				"`bridge_id` is different than calculated from `bridge_origin_universal_location_as_latest` and `bridge_destination_universal_location_as_latest`, needs migration!"
			);

			// check bridge account owner
			ensure!(
				T::BridgeOriginAccountIdConverter::convert_location(bridge_origin_relative_location_as_latest) == Some(bridge.bridge_owner_account),
				"`bridge.bridge_owner_account` is different than calculated from `bridge.bridge_origin_relative_location`, needs migration!"
			);

			Ok(bridge.lane_id)
		}

		/// Ensure the correctness of the state of the connected `pallet_bridge_messages` instance.
		pub fn do_try_state_for_messages() -> Result<(), sp_runtime::TryRuntimeError> {
			// check that all `InboundLanes` laneIds have mapping to some bridge.
			for lane_id in pallet_bridge_messages::InboundLanes::<T, T::BridgeMessagesPalletInstance>::iter_keys() {
				log::info!(target: LOG_TARGET, "Checking `do_try_state_for_messages` for `InboundLanes`'s lane_id: {lane_id:?}...");
				ensure!(
					LaneToBridge::<T, I>::get(lane_id).is_some(),
					"Found `LaneToBridge` inconsistency for `InboundLanes`'s lane_id - missing mapping!"
				);
			}

			// check that all `OutboundLanes` laneIds have mapping to some bridge.
			for lane_id in pallet_bridge_messages::OutboundLanes::<T, T::BridgeMessagesPalletInstance>::iter_keys() {
				log::info!(target: LOG_TARGET, "Checking `do_try_state_for_messages` for `OutboundLanes`'s lane_id: {lane_id:?}...");
				ensure!(
					LaneToBridge::<T, I>::get(lane_id).is_some(),
					"Found `LaneToBridge` inconsistency for `OutboundLanes`'s lane_id - missing mapping!"
				);
			}

			Ok(())
		}
	}

	/// All registered bridges.
	#[pallet::storage]
	pub type Bridges<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Identity, BridgeId, BridgeOf<T, I>>;
	/// All registered `lane_id` and `bridge_id` mappings.
	#[pallet::storage]
	pub type LaneToBridge<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Identity, T::LaneId, BridgeId>;

	#[pallet::genesis_config]
	#[derive(DefaultNoBound)]
	pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
		/// Opened bridges.
		///
		/// Keep in mind that we are **NOT** reserving any amount for the bridges opened at
		/// genesis. We are **NOT** opening lanes, used by this bridge. It all must be done using
		/// other pallets genesis configuration or some other means.
		pub opened_bridges: Vec<(Location, InteriorLocation, Option<T::LaneId>)>,
		/// Dummy marker.
		#[serde(skip)]
		pub _phantom: sp_std::marker::PhantomData<(T, I)>,
	}

	#[pallet::genesis_build]
	impl<T: Config<I>, I: 'static> BuildGenesisConfig for GenesisConfig<T, I>
	where
		T: frame_system::Config<AccountId = AccountIdOf<ThisChainOf<T, I>>>,
	{
		fn build(&self) {
			for (
				bridge_origin_relative_location,
				bridge_destination_universal_location,
				maybe_lane_id,
			) in &self.opened_bridges
			{
				let locations = Pallet::<T, I>::bridge_locations(
					bridge_origin_relative_location.clone(),
					bridge_destination_universal_location.clone().into(),
				)
				.expect("Invalid genesis configuration");

				let lane_id = match maybe_lane_id {
					Some(lane_id) => *lane_id,
					None =>
						locations.calculate_lane_id(xcm::latest::VERSION).expect("Valid locations"),
				};

				Pallet::<T, I>::do_open_bridge(locations, lane_id, true)
					.expect("Valid opened bridge!");
			}
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// The bridge between two locations has been opened.
		BridgeOpened {
			/// Bridge identifier.
			bridge_id: BridgeId,
			/// Amount of deposit held.
			bridge_deposit: BalanceOf<ThisChainOf<T, I>>,

			/// Universal location of local bridge endpoint.
			local_endpoint: Box<InteriorLocation>,
			/// Universal location of remote bridge endpoint.
			remote_endpoint: Box<InteriorLocation>,
			/// Lane identifier.
			lane_id: T::LaneId,
		},
		/// Bridge is going to be closed, but not yet fully pruned from the runtime storage.
		ClosingBridge {
			/// Bridge identifier.
			bridge_id: BridgeId,
			/// Lane identifier.
			lane_id: T::LaneId,
			/// Number of pruned messages during the close call.
			pruned_messages: MessageNonce,
			/// Number of enqueued messages that need to be pruned in follow up calls.
			enqueued_messages: MessageNonce,
		},
		/// Bridge has been closed and pruned from the runtime storage. It now may be reopened
		/// again by any participant.
		BridgePruned {
			/// Bridge identifier.
			bridge_id: BridgeId,
			/// Lane identifier.
			lane_id: T::LaneId,
			/// Amount of deposit released.
			bridge_deposit: BalanceOf<ThisChainOf<T, I>>,
			/// Number of pruned messages during the close call.
			pruned_messages: MessageNonce,
		},
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Bridge locations error.
		BridgeLocations(BridgeLocationsError),
		/// Invalid local bridge origin account.
		InvalidBridgeOriginAccount,
		/// The bridge is already registered in this pallet.
		BridgeAlreadyExists,
		/// The local origin already owns a maximal number of bridges.
		TooManyBridgesForLocalOrigin,
		/// Trying to close already closed bridge.
		BridgeAlreadyClosed,
		/// Lanes manager error.
		LanesManager(LanesManagerError),
		/// Trying to access unknown bridge.
		UnknownBridge,
		/// The bridge origin can't pay the required amount for opening the bridge.
		FailedToReserveBridgeDeposit,
		/// The version of XCM location argument is unsupported.
		UnsupportedXcmVersion,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bp_messages::LaneIdType;
	use mock::*;

	use frame_support::{assert_err, assert_noop, assert_ok, traits::fungible::Mutate, BoundedVec};
	use frame_system::{EventRecord, Phase};
	use sp_runtime::TryRuntimeError;

	fn fund_origin_sovereign_account(locations: &BridgeLocations, balance: Balance) -> AccountId {
		let bridge_owner_account =
			LocationToAccountId::convert_location(locations.bridge_origin_relative_location())
				.unwrap();
		assert_ok!(Balances::mint_into(&bridge_owner_account, balance));
		bridge_owner_account
	}

	fn mock_open_bridge_from_with(
		origin: RuntimeOrigin,
		deposit: Balance,
		with: InteriorLocation,
	) -> (BridgeOf<TestRuntime, ()>, BridgeLocations) {
		let locations =
			XcmOverBridge::bridge_locations_from_origin(origin, Box::new(with.into())).unwrap();
		let lane_id = locations.calculate_lane_id(xcm::latest::VERSION).unwrap();
		let bridge_owner_account =
			fund_origin_sovereign_account(&locations, deposit + ExistentialDeposit::get());
		Balances::hold(&HoldReason::BridgeDeposit.into(), &bridge_owner_account, deposit).unwrap();

		let bridge = Bridge {
			bridge_origin_relative_location: Box::new(
				locations.bridge_origin_relative_location().clone().into(),
			),
			bridge_origin_universal_location: Box::new(
				locations.bridge_origin_universal_location().clone().into(),
			),
			bridge_destination_universal_location: Box::new(
				locations.bridge_destination_universal_location().clone().into(),
			),
			state: BridgeState::Opened,
			bridge_owner_account,
			deposit,
			lane_id,
		};
		Bridges::<TestRuntime, ()>::insert(locations.bridge_id(), bridge.clone());
		LaneToBridge::<TestRuntime, ()>::insert(bridge.lane_id, locations.bridge_id());

		let lanes_manager = LanesManagerOf::<TestRuntime, ()>::new();
		lanes_manager.create_inbound_lane(bridge.lane_id).unwrap();
		lanes_manager.create_outbound_lane(bridge.lane_id).unwrap();

		assert_ok!(XcmOverBridge::do_try_state());

		(bridge, *locations)
	}

	fn mock_open_bridge_from(
		origin: RuntimeOrigin,
		deposit: Balance,
	) -> (BridgeOf<TestRuntime, ()>, BridgeLocations) {
		mock_open_bridge_from_with(origin, deposit, bridged_asset_hub_universal_location())
	}

	fn enqueue_message(lane: TestLaneIdType) {
		let lanes_manager = LanesManagerOf::<TestRuntime, ()>::new();
		lanes_manager
			.active_outbound_lane(lane)
			.unwrap()
			.send_message(BoundedVec::try_from(vec![42]).expect("We craft valid messages"));
	}

	#[test]
	fn open_bridge_fails_if_origin_is_not_allowed() {
		run_test(|| {
			assert_noop!(
				XcmOverBridge::open_bridge(
					OpenBridgeOrigin::disallowed_origin(),
					Box::new(bridged_asset_hub_universal_location().into()),
				),
				sp_runtime::DispatchError::BadOrigin,
			);
		})
	}

	#[test]
	fn open_bridge_fails_if_origin_is_not_relative() {
		run_test(|| {
			assert_noop!(
				XcmOverBridge::open_bridge(
					OpenBridgeOrigin::parent_relay_chain_universal_origin(),
					Box::new(bridged_asset_hub_universal_location().into()),
				),
				Error::<TestRuntime, ()>::BridgeLocations(
					BridgeLocationsError::InvalidBridgeOrigin
				),
			);

			assert_noop!(
				XcmOverBridge::open_bridge(
					OpenBridgeOrigin::sibling_parachain_universal_origin(),
					Box::new(bridged_asset_hub_universal_location().into()),
				),
				Error::<TestRuntime, ()>::BridgeLocations(
					BridgeLocationsError::InvalidBridgeOrigin
				),
			);
		})
	}

	#[test]
	fn open_bridge_fails_if_destination_is_not_remote() {
		run_test(|| {
			assert_noop!(
				XcmOverBridge::open_bridge(
					OpenBridgeOrigin::parent_relay_chain_origin(),
					Box::new(
						[GlobalConsensus(RelayNetwork::get()), Parachain(BRIDGED_ASSET_HUB_ID)]
							.into()
					),
				),
				Error::<TestRuntime, ()>::BridgeLocations(BridgeLocationsError::DestinationIsLocal),
			);
		});
	}

	#[test]
	fn open_bridge_fails_if_outside_of_bridged_consensus() {
		run_test(|| {
			assert_noop!(
				XcmOverBridge::open_bridge(
					OpenBridgeOrigin::parent_relay_chain_origin(),
					Box::new(
						[
							GlobalConsensus(NonBridgedRelayNetwork::get()),
							Parachain(BRIDGED_ASSET_HUB_ID)
						]
						.into()
					),
				),
				Error::<TestRuntime, ()>::BridgeLocations(
					BridgeLocationsError::UnreachableDestination
				),
			);
		});
	}

	#[test]
	fn open_bridge_fails_if_origin_has_no_sovereign_account() {
		run_test(|| {
			assert_noop!(
				XcmOverBridge::open_bridge(
					OpenBridgeOrigin::origin_without_sovereign_account(),
					Box::new(bridged_asset_hub_universal_location().into()),
				),
				Error::<TestRuntime, ()>::InvalidBridgeOriginAccount,
			);
		});
	}

	#[test]
	fn open_bridge_fails_if_origin_sovereign_account_has_no_enough_funds() {
		run_test(|| {
			assert_noop!(
				XcmOverBridge::open_bridge(
					OpenBridgeOrigin::sibling_parachain_origin(),
					Box::new(bridged_asset_hub_universal_location().into()),
				),
				Error::<TestRuntime, ()>::FailedToReserveBridgeDeposit,
			);
		});
	}

	#[test]
	fn open_bridge_fails_if_it_already_exists() {
		run_test(|| {
			let origin = OpenBridgeOrigin::parent_relay_chain_origin();
			let locations = XcmOverBridge::bridge_locations_from_origin(
				origin.clone(),
				Box::new(bridged_asset_hub_universal_location().into()),
			)
			.unwrap();
			let lane_id = locations.calculate_lane_id(xcm::latest::VERSION).unwrap();
			fund_origin_sovereign_account(
				&locations,
				BridgeDeposit::get() + ExistentialDeposit::get(),
			);

			Bridges::<TestRuntime, ()>::insert(
				locations.bridge_id(),
				Bridge {
					bridge_origin_relative_location: Box::new(
						locations.bridge_origin_relative_location().clone().into(),
					),
					bridge_origin_universal_location: Box::new(
						locations.bridge_origin_universal_location().clone().into(),
					),
					bridge_destination_universal_location: Box::new(
						locations.bridge_destination_universal_location().clone().into(),
					),
					state: BridgeState::Opened,
					bridge_owner_account: [0u8; 32].into(),
					deposit: 0,
					lane_id,
				},
			);

			assert_noop!(
				XcmOverBridge::open_bridge(
					origin,
					Box::new(bridged_asset_hub_universal_location().into()),
				),
				Error::<TestRuntime, ()>::BridgeAlreadyExists,
			);
		})
	}

	#[test]
	fn open_bridge_fails_if_its_lanes_already_exists() {
		run_test(|| {
			let origin = OpenBridgeOrigin::parent_relay_chain_origin();
			let locations = XcmOverBridge::bridge_locations_from_origin(
				origin.clone(),
				Box::new(bridged_asset_hub_universal_location().into()),
			)
			.unwrap();
			let lane_id = locations.calculate_lane_id(xcm::latest::VERSION).unwrap();
			fund_origin_sovereign_account(
				&locations,
				BridgeDeposit::get() + ExistentialDeposit::get(),
			);

			let lanes_manager = LanesManagerOf::<TestRuntime, ()>::new();

			lanes_manager.create_inbound_lane(lane_id).unwrap();
			assert_noop!(
				XcmOverBridge::open_bridge(
					origin.clone(),
					Box::new(bridged_asset_hub_universal_location().into()),
				),
				Error::<TestRuntime, ()>::LanesManager(LanesManagerError::InboundLaneAlreadyExists),
			);

			lanes_manager.active_inbound_lane(lane_id).unwrap().purge();
			lanes_manager.create_outbound_lane(lane_id).unwrap();
			assert_noop!(
				XcmOverBridge::open_bridge(
					origin,
					Box::new(bridged_asset_hub_universal_location().into()),
				),
				Error::<TestRuntime, ()>::LanesManager(
					LanesManagerError::OutboundLaneAlreadyExists
				),
			);
		})
	}

	#[test]
	fn open_bridge_works() {
		run_test(|| {
			// in our test runtime, we expect that bridge may be opened by parent relay chain
			// and any sibling parachain
			let origins = [
				(OpenBridgeOrigin::parent_relay_chain_origin(), 0),
				(OpenBridgeOrigin::sibling_parachain_origin(), BridgeDeposit::get()),
			];

			// check that every origin may open the bridge
			let lanes_manager = LanesManagerOf::<TestRuntime, ()>::new();
			let existential_deposit = ExistentialDeposit::get();
			for (origin, expected_deposit) in origins {
				// reset events
				System::set_block_number(1);
				System::reset_events();

				// compute all other locations
				let xcm_version = xcm::latest::VERSION;
				let locations = XcmOverBridge::bridge_locations_from_origin(
					origin.clone(),
					Box::new(
						VersionedInteriorLocation::from(bridged_asset_hub_universal_location())
							.into_version(xcm_version)
							.expect("valid conversion"),
					),
				)
				.unwrap();
				let lane_id = locations.calculate_lane_id(xcm_version).unwrap();

				// ensure that there's no bridge and lanes in the storage
				assert_eq!(Bridges::<TestRuntime, ()>::get(locations.bridge_id()), None);
				assert_eq!(
					lanes_manager.active_inbound_lane(lane_id).map(drop),
					Err(LanesManagerError::UnknownInboundLane)
				);
				assert_eq!(
					lanes_manager.active_outbound_lane(lane_id).map(drop),
					Err(LanesManagerError::UnknownOutboundLane)
				);
				assert_eq!(LaneToBridge::<TestRuntime, ()>::get(lane_id), None);

				// give enough funds to the sovereign account of the bridge origin
				let bridge_owner_account = fund_origin_sovereign_account(
					&locations,
					expected_deposit + existential_deposit,
				);
				assert_eq!(
					Balances::free_balance(&bridge_owner_account),
					expected_deposit + existential_deposit
				);
				assert_eq!(Balances::reserved_balance(&bridge_owner_account), 0);

				// now open the bridge
				assert_ok!(XcmOverBridge::open_bridge(
					origin,
					Box::new(locations.bridge_destination_universal_location().clone().into()),
				));

				// ensure that everything has been set up in the runtime storage
				assert_eq!(
					Bridges::<TestRuntime, ()>::get(locations.bridge_id()),
					Some(Bridge {
						bridge_origin_relative_location: Box::new(
							locations.bridge_origin_relative_location().clone().into()
						),
						bridge_origin_universal_location: Box::new(
							locations.bridge_origin_universal_location().clone().into(),
						),
						bridge_destination_universal_location: Box::new(
							locations.bridge_destination_universal_location().clone().into(),
						),
						state: BridgeState::Opened,
						bridge_owner_account: bridge_owner_account.clone(),
						deposit: expected_deposit,
						lane_id
					}),
				);
				assert_eq!(
					lanes_manager.active_inbound_lane(lane_id).map(|l| l.state()),
					Ok(LaneState::Opened)
				);
				assert_eq!(
					lanes_manager.active_outbound_lane(lane_id).map(|l| l.state()),
					Ok(LaneState::Opened)
				);
				assert_eq!(
					LaneToBridge::<TestRuntime, ()>::get(lane_id),
					Some(*locations.bridge_id())
				);
				assert_eq!(Balances::free_balance(&bridge_owner_account), existential_deposit);
				assert_eq!(Balances::reserved_balance(&bridge_owner_account), expected_deposit);

				// ensure that the proper event is deposited
				assert_eq!(
					System::events().last(),
					Some(&EventRecord {
						phase: Phase::Initialization,
						event: RuntimeEvent::XcmOverBridge(Event::BridgeOpened {
							bridge_id: *locations.bridge_id(),
							bridge_deposit: expected_deposit,
							local_endpoint: Box::new(
								locations.bridge_origin_universal_location().clone()
							),
							remote_endpoint: Box::new(
								locations.bridge_destination_universal_location().clone()
							),
							lane_id: lane_id.into()
						}),
						topics: vec![],
					}),
				);

				// check state
				assert_ok!(XcmOverBridge::do_try_state());
			}
		});
	}

	#[test]
	fn close_bridge_fails_if_origin_is_not_allowed() {
		run_test(|| {
			assert_noop!(
				XcmOverBridge::close_bridge(
					OpenBridgeOrigin::disallowed_origin(),
					Box::new(bridged_asset_hub_universal_location().into()),
					0,
				),
				sp_runtime::DispatchError::BadOrigin,
			);
		})
	}

	#[test]
	fn close_bridge_fails_if_origin_is_not_relative() {
		run_test(|| {
			assert_noop!(
				XcmOverBridge::close_bridge(
					OpenBridgeOrigin::parent_relay_chain_universal_origin(),
					Box::new(bridged_asset_hub_universal_location().into()),
					0,
				),
				Error::<TestRuntime, ()>::BridgeLocations(
					BridgeLocationsError::InvalidBridgeOrigin
				),
			);

			assert_noop!(
				XcmOverBridge::close_bridge(
					OpenBridgeOrigin::sibling_parachain_universal_origin(),
					Box::new(bridged_asset_hub_universal_location().into()),
					0,
				),
				Error::<TestRuntime, ()>::BridgeLocations(
					BridgeLocationsError::InvalidBridgeOrigin
				),
			);
		})
	}

	#[test]
	fn close_bridge_fails_if_its_lanes_are_unknown() {
		run_test(|| {
			let origin = OpenBridgeOrigin::parent_relay_chain_origin();
			let (bridge, locations) = mock_open_bridge_from(origin.clone(), 0);

			let lanes_manager = LanesManagerOf::<TestRuntime, ()>::new();
			lanes_manager.any_state_inbound_lane(bridge.lane_id).unwrap().purge();
			assert_noop!(
				XcmOverBridge::close_bridge(
					origin.clone(),
					Box::new(locations.bridge_destination_universal_location().clone().into()),
					0,
				),
				Error::<TestRuntime, ()>::LanesManager(LanesManagerError::UnknownInboundLane),
			);
			lanes_manager.any_state_outbound_lane(bridge.lane_id).unwrap().purge();

			let (_, locations) = mock_open_bridge_from(origin.clone(), 0);
			lanes_manager.any_state_outbound_lane(bridge.lane_id).unwrap().purge();
			assert_noop!(
				XcmOverBridge::close_bridge(
					origin,
					Box::new(locations.bridge_destination_universal_location().clone().into()),
					0,
				),
				Error::<TestRuntime, ()>::LanesManager(LanesManagerError::UnknownOutboundLane),
			);
		});
	}

	#[test]
	fn close_bridge_works() {
		run_test(|| {
			let origin = OpenBridgeOrigin::parent_relay_chain_origin();
			let expected_deposit = BridgeDeposit::get();
			let (bridge, locations) = mock_open_bridge_from(origin.clone(), expected_deposit);
			System::set_block_number(1);

			// remember owner balances
			let free_balance = Balances::free_balance(&bridge.bridge_owner_account);
			let reserved_balance = Balances::reserved_balance(&bridge.bridge_owner_account);

			// enqueue some messages
			for _ in 0..32 {
				enqueue_message(bridge.lane_id);
			}

			// now call the `close_bridge`, which will only partially prune messages
			assert_ok!(XcmOverBridge::close_bridge(
				origin.clone(),
				Box::new(locations.bridge_destination_universal_location().clone().into()),
				16,
			),);

			// as a result, the bridge and lanes are switched to the `Closed` state, some messages
			// are pruned, but funds are not unreserved
			let lanes_manager = LanesManagerOf::<TestRuntime, ()>::new();
			assert_eq!(
				Bridges::<TestRuntime, ()>::get(locations.bridge_id()).map(|b| b.state),
				Some(BridgeState::Closed)
			);
			assert_eq!(
				lanes_manager.any_state_inbound_lane(bridge.lane_id).unwrap().state(),
				LaneState::Closed
			);
			assert_eq!(
				lanes_manager.any_state_outbound_lane(bridge.lane_id).unwrap().state(),
				LaneState::Closed
			);
			assert_eq!(
				lanes_manager
					.any_state_outbound_lane(bridge.lane_id)
					.unwrap()
					.queued_messages()
					.checked_len(),
				Some(16)
			);
			assert_eq!(
				LaneToBridge::<TestRuntime, ()>::get(bridge.lane_id),
				Some(*locations.bridge_id())
			);
			assert_eq!(Balances::free_balance(&bridge.bridge_owner_account), free_balance);
			assert_eq!(Balances::reserved_balance(&bridge.bridge_owner_account), reserved_balance);
			assert_eq!(
				System::events().last(),
				Some(&EventRecord {
					phase: Phase::Initialization,
					event: RuntimeEvent::XcmOverBridge(Event::ClosingBridge {
						bridge_id: *locations.bridge_id(),
						lane_id: bridge.lane_id.into(),
						pruned_messages: 16,
						enqueued_messages: 16,
					}),
					topics: vec![],
				}),
			);

			// now call the `close_bridge` again, which will only partially prune messages
			assert_ok!(XcmOverBridge::close_bridge(
				origin.clone(),
				Box::new(locations.bridge_destination_universal_location().clone().into()),
				8,
			),);

			// nothing is changed (apart from the pruned messages)
			assert_eq!(
				Bridges::<TestRuntime, ()>::get(locations.bridge_id()).map(|b| b.state),
				Some(BridgeState::Closed)
			);
			assert_eq!(
				lanes_manager.any_state_inbound_lane(bridge.lane_id).unwrap().state(),
				LaneState::Closed
			);
			assert_eq!(
				lanes_manager.any_state_outbound_lane(bridge.lane_id).unwrap().state(),
				LaneState::Closed
			);
			assert_eq!(
				lanes_manager
					.any_state_outbound_lane(bridge.lane_id)
					.unwrap()
					.queued_messages()
					.checked_len(),
				Some(8)
			);
			assert_eq!(
				LaneToBridge::<TestRuntime, ()>::get(bridge.lane_id),
				Some(*locations.bridge_id())
			);
			assert_eq!(Balances::free_balance(&bridge.bridge_owner_account), free_balance);
			assert_eq!(Balances::reserved_balance(&bridge.bridge_owner_account), reserved_balance);
			assert_eq!(
				System::events().last(),
				Some(&EventRecord {
					phase: Phase::Initialization,
					event: RuntimeEvent::XcmOverBridge(Event::ClosingBridge {
						bridge_id: *locations.bridge_id(),
						lane_id: bridge.lane_id.into(),
						pruned_messages: 8,
						enqueued_messages: 8,
					}),
					topics: vec![],
				}),
			);

			// now call the `close_bridge` again that will prune all remaining messages and the
			// bridge
			assert_ok!(XcmOverBridge::close_bridge(
				origin,
				Box::new(locations.bridge_destination_universal_location().clone().into()),
				9,
			),);

			// there's no traces of bridge in the runtime storage and funds are unreserved
			assert_eq!(
				Bridges::<TestRuntime, ()>::get(locations.bridge_id()).map(|b| b.state),
				None
			);
			assert_eq!(
				lanes_manager.any_state_inbound_lane(bridge.lane_id).map(drop),
				Err(LanesManagerError::UnknownInboundLane)
			);
			assert_eq!(
				lanes_manager.any_state_outbound_lane(bridge.lane_id).map(drop),
				Err(LanesManagerError::UnknownOutboundLane)
			);
			assert_eq!(LaneToBridge::<TestRuntime, ()>::get(bridge.lane_id), None);
			assert_eq!(
				Balances::free_balance(&bridge.bridge_owner_account),
				free_balance + reserved_balance
			);
			assert_eq!(Balances::reserved_balance(&bridge.bridge_owner_account), 0);
			assert_eq!(
				System::events().last(),
				Some(&EventRecord {
					phase: Phase::Initialization,
					event: RuntimeEvent::XcmOverBridge(Event::BridgePruned {
						bridge_id: *locations.bridge_id(),
						lane_id: bridge.lane_id.into(),
						bridge_deposit: expected_deposit,
						pruned_messages: 8,
					}),
					topics: vec![],
				}),
			);
		});
	}

	#[test]
	fn do_try_state_works() {
		let bridge_origin_relative_location = SiblingLocation::get();
		let bridge_origin_universal_location = SiblingUniversalLocation::get();
		let bridge_destination_universal_location = BridgedUniversalDestination::get();
		let bridge_owner_account =
			LocationToAccountId::convert_location(&bridge_origin_relative_location)
				.expect("valid accountId");
		let bridge_owner_account_mismatch =
			LocationToAccountId::convert_location(&Location::parent()).expect("valid accountId");
		let bridge_id = BridgeId::new(
			&bridge_origin_universal_location,
			&bridge_destination_universal_location,
		);
		let bridge_id_mismatch = BridgeId::new(&InteriorLocation::Here, &InteriorLocation::Here);
		let lane_id = TestLaneIdType::try_new(1, 2).unwrap();
		let lane_id_mismatch = TestLaneIdType::try_new(3, 4).unwrap();

		let test_bridge_state = |id,
		                         bridge,
		                         (lane_id, bridge_id),
		                         (inbound_lane_id, outbound_lane_id),
		                         expected_error: Option<TryRuntimeError>| {
			Bridges::<TestRuntime, ()>::insert(id, bridge);
			LaneToBridge::<TestRuntime, ()>::insert(lane_id, bridge_id);

			let lanes_manager = LanesManagerOf::<TestRuntime, ()>::new();
			lanes_manager.create_inbound_lane(inbound_lane_id).unwrap();
			lanes_manager.create_outbound_lane(outbound_lane_id).unwrap();

			let result = XcmOverBridge::do_try_state();
			if let Some(e) = expected_error {
				assert_err!(result, e);
			} else {
				assert_ok!(result);
			}
		};
		let cleanup = |bridge_id, lane_ids| {
			Bridges::<TestRuntime, ()>::remove(bridge_id);
			for lane_id in lane_ids {
				LaneToBridge::<TestRuntime, ()>::remove(lane_id);
				let lanes_manager = LanesManagerOf::<TestRuntime, ()>::new();
				if let Ok(lane) = lanes_manager.any_state_inbound_lane(lane_id) {
					lane.purge();
				}
				if let Ok(lane) = lanes_manager.any_state_outbound_lane(lane_id) {
					lane.purge();
				}
			}
			assert_ok!(XcmOverBridge::do_try_state());
		};

		run_test(|| {
			// ok state
			test_bridge_state(
				bridge_id,
				Bridge {
					bridge_origin_relative_location: Box::new(VersionedLocation::from(
						bridge_origin_relative_location.clone(),
					)),
					bridge_origin_universal_location: Box::new(VersionedInteriorLocation::from(
						bridge_origin_universal_location.clone(),
					)),
					bridge_destination_universal_location: Box::new(
						VersionedInteriorLocation::from(
							bridge_destination_universal_location.clone(),
						),
					),
					state: BridgeState::Opened,
					bridge_owner_account: bridge_owner_account.clone(),
					deposit: Zero::zero(),
					lane_id,
				},
				(lane_id, bridge_id),
				(lane_id, lane_id),
				None,
			);
			cleanup(bridge_id, vec![lane_id]);

			// error - missing `LaneToBridge` mapping
			test_bridge_state(
				bridge_id,
				Bridge {
					bridge_origin_relative_location: Box::new(VersionedLocation::from(
						bridge_origin_relative_location.clone(),
					)),
					bridge_origin_universal_location: Box::new(VersionedInteriorLocation::from(
						bridge_origin_universal_location.clone(),
					)),
					bridge_destination_universal_location: Box::new(
						VersionedInteriorLocation::from(
							bridge_destination_universal_location.clone(),
						),
					),
					state: BridgeState::Opened,
					bridge_owner_account: bridge_owner_account.clone(),
					deposit: Zero::zero(),
					lane_id,
				},
				(lane_id, bridge_id_mismatch),
				(lane_id, lane_id),
				Some(TryRuntimeError::Other(
					"Found `LaneToBridge` inconsistency for bridge_id - missing mapping!",
				)),
			);
			cleanup(bridge_id, vec![lane_id]);

			// error bridge owner account cannot be calculated
			test_bridge_state(
				bridge_id,
				Bridge {
					bridge_origin_relative_location: Box::new(VersionedLocation::from(
						bridge_origin_relative_location.clone(),
					)),
					bridge_origin_universal_location: Box::new(VersionedInteriorLocation::from(
						bridge_origin_universal_location.clone(),
					)),
					bridge_destination_universal_location: Box::new(VersionedInteriorLocation::from(
						bridge_destination_universal_location.clone(),
					)),
					state: BridgeState::Opened,
					bridge_owner_account: bridge_owner_account_mismatch.clone(),
					deposit: Zero::zero(),
					lane_id,
				},
				(lane_id, bridge_id),
				(lane_id, lane_id),
				Some(TryRuntimeError::Other("`bridge.bridge_owner_account` is different than calculated from `bridge.bridge_origin_relative_location`, needs migration!")),
			);
			cleanup(bridge_id, vec![lane_id]);

			// error when (bridge_origin_universal_location + bridge_destination_universal_location)
			// produces different `BridgeId`
			test_bridge_state(
				bridge_id_mismatch,
				Bridge {
					bridge_origin_relative_location: Box::new(VersionedLocation::from(
						bridge_origin_relative_location.clone(),
					)),
					bridge_origin_universal_location: Box::new(VersionedInteriorLocation::from(
						bridge_origin_universal_location.clone(),
					)),
					bridge_destination_universal_location: Box::new(VersionedInteriorLocation::from(
						bridge_destination_universal_location.clone(),
					)),
					state: BridgeState::Opened,
					bridge_owner_account: bridge_owner_account_mismatch.clone(),
					deposit: Zero::zero(),
					lane_id,
				},
				(lane_id, bridge_id_mismatch),
				(lane_id, lane_id),
				Some(TryRuntimeError::Other("`bridge_id` is different than calculated from `bridge_origin_universal_location_as_latest` and `bridge_destination_universal_location_as_latest`, needs migration!")),
			);
			cleanup(bridge_id_mismatch, vec![lane_id]);

			// missing inbound lane for a bridge
			test_bridge_state(
				bridge_id,
				Bridge {
					bridge_origin_relative_location: Box::new(VersionedLocation::from(
						bridge_origin_relative_location.clone(),
					)),
					bridge_origin_universal_location: Box::new(VersionedInteriorLocation::from(
						bridge_origin_universal_location.clone(),
					)),
					bridge_destination_universal_location: Box::new(
						VersionedInteriorLocation::from(
							bridge_destination_universal_location.clone(),
						),
					),
					state: BridgeState::Opened,
					bridge_owner_account: bridge_owner_account.clone(),
					deposit: Zero::zero(),
					lane_id,
				},
				(lane_id, bridge_id),
				(lane_id_mismatch, lane_id),
				Some(TryRuntimeError::Other("Inbound lane not found!")),
			);
			cleanup(bridge_id, vec![lane_id, lane_id_mismatch]);

			// missing outbound lane for a bridge
			test_bridge_state(
				bridge_id,
				Bridge {
					bridge_origin_relative_location: Box::new(VersionedLocation::from(
						bridge_origin_relative_location.clone(),
					)),
					bridge_origin_universal_location: Box::new(VersionedInteriorLocation::from(
						bridge_origin_universal_location.clone(),
					)),
					bridge_destination_universal_location: Box::new(
						VersionedInteriorLocation::from(
							bridge_destination_universal_location.clone(),
						),
					),
					state: BridgeState::Opened,
					bridge_owner_account: bridge_owner_account.clone(),
					deposit: Zero::zero(),
					lane_id,
				},
				(lane_id, bridge_id),
				(lane_id, lane_id_mismatch),
				Some(TryRuntimeError::Other("Outbound lane not found!")),
			);
			cleanup(bridge_id, vec![lane_id, lane_id_mismatch]);

			// missing bridge for inbound lane
			let lanes_manager = LanesManagerOf::<TestRuntime, ()>::new();
			assert!(lanes_manager.create_inbound_lane(lane_id).is_ok());
			assert_err!(XcmOverBridge::do_try_state(), TryRuntimeError::Other("Found `LaneToBridge` inconsistency for `InboundLanes`'s lane_id - missing mapping!"));
			cleanup(bridge_id, vec![lane_id]);

			// missing bridge for outbound lane
			let lanes_manager = LanesManagerOf::<TestRuntime, ()>::new();
			assert!(lanes_manager.create_outbound_lane(lane_id).is_ok());
			assert_err!(XcmOverBridge::do_try_state(), TryRuntimeError::Other("Found `LaneToBridge` inconsistency for `OutboundLanes`'s lane_id - missing mapping!"));
			cleanup(bridge_id, vec![lane_id]);
		});
	}

	#[test]
	fn ensure_encoding_compatibility() {
		use codec::Encode;

		let bridge_destination_universal_location = BridgedUniversalDestination::get();
		let may_prune_messages = 13;

		assert_eq!(
			bp_xcm_bridge_hub::XcmBridgeHubCall::open_bridge {
				bridge_destination_universal_location: Box::new(
					bridge_destination_universal_location.clone().into()
				)
			}
			.encode(),
			Call::<TestRuntime, ()>::open_bridge {
				bridge_destination_universal_location: Box::new(
					bridge_destination_universal_location.clone().into()
				)
			}
			.encode()
		);
		assert_eq!(
			bp_xcm_bridge_hub::XcmBridgeHubCall::close_bridge {
				bridge_destination_universal_location: Box::new(
					bridge_destination_universal_location.clone().into()
				),
				may_prune_messages,
			}
			.encode(),
			Call::<TestRuntime, ()>::close_bridge {
				bridge_destination_universal_location: Box::new(
					bridge_destination_universal_location.clone().into()
				),
				may_prune_messages,
			}
			.encode()
		);
	}
>>>>>>> 710e74d (Bridges lane id agnostic for backwards compatibility (#5649))
}
