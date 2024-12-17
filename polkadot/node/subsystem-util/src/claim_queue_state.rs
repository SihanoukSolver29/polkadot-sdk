// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! `ClaimQueueState` tracks the state of the claim queue over a set of relay blocks. Refer to
//! [`ClaimQueueState`] for more details.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::LOG_TARGET;
use polkadot_primitives::{Hash, Id as ParaId};

/// Represents the state of a claim.
#[derive(Debug, PartialEq)]
enum ClaimState {
	/// Unclaimed
	Free,
	/// The candidate is pending fetching or validation.
	Pending(Option<Hash>),
	/// The candidate is seconded.
	Seconded(Hash),
}

/// Represents a single claim from the claim queue, mapped to the relay chain block where it could
/// be backed on-chain.
#[derive(Debug, PartialEq)]
struct ClaimInfo {
	// Hash of the relay chain block. Can be `None` if it is still not known (a future block).
	hash: Option<Hash>,
	/// Represents the `ParaId` scheduled for the block. Can be `None` if nothing is scheduled.
	claim: Option<ParaId>,
	/// The length of the claim queue at the block. It is used to determine the 'block window'
	/// where a claim can be made.
	claim_queue_len: usize,
	claimed: ClaimState,
}

/// Tracks the state of the claim queue over a set of relay blocks.
///
/// Generally the claim queue represents the `ParaId` that should be scheduled at the current block
/// (the first element of the claim queue) and N other `ParaId`s which are supposed to be scheduled
/// on the next relay blocks. In other words the claim queue is a rolling window giving a hint what
/// should be built/fetched/accepted (depending on the context) at each block.
///
/// Since the claim queue peeks into the future blocks there is a relation between the claim queue
/// state between the current block and the future blocks.
/// Let's see an example with 2 co-scheduled parachains:
/// - relay parent 1; Claim queue: [A, B, A]
/// - relay parent 2; Claim queue: [B, A, B]
/// - relay parent 3; Claim queue: [A, B, A]
/// - and so on
///
/// Note that at rp1 the second element in the claim queue is equal to the first one in rp2. Also
/// the third element of the claim queue at rp1 is equal to the second one in rp2 and the first one
/// in rp3.
///
/// So if we want to claim the third slot at rp 1 we are also claiming the second at rp2 and first
/// at rp3. To track this in a simple way we can project the claim queue onto the relay blocks like
/// this:
///               [A]   [B]   [A] -> this is the claim queue at rp3
///         [B]   [A]   [B]       -> this is the claim queue at rp2
///   [A]   [B]   [A]	          -> this is the claim queue at rp1
/// [RP 1][RP 2][RP 3][RP X][RP Y] -> relay blocks, RP x and RP Y are future blocks
///
/// Note that the claims at each column are the same so we can simplify this by just projecting a
/// single claim over a block:
///   [A]   [B]   [A]   [B]   [A]  -> claims effectively are the same
/// [RP 1][RP 2][RP 3][RP X][RP Y] -> relay blocks, RP x and RP Y are future blocks
///
/// Basically this is how `ClaimQueueState` works. It keeps track of claims at each block by mapping
/// claims to relay blocks.
///
/// How making a claim works?
/// At each relay block we keep track how long is the claim queue. This is a 'window' where we can
/// make a claim. So adding a claim just looks for a free spot at this window and claims it.
///
/// Note on adding a new leaf.
/// When a new leaf is added we check if the first element in its claim queue matches with the
/// projection on the first element in 'future blocks'. If yes - the new relay block inherits this
/// claim. If not - this means that the claim queue changed for some reason so the claim can't be
/// inherited. This should not happen under normal circumstances. But if it happens it means that we
/// have got one claim which won't be satisfied in the worst case scenario.
#[derive(Debug)]
pub struct ClaimQueueState {
	block_state: VecDeque<ClaimInfo>,
	future_blocks: VecDeque<ClaimInfo>,
	/// Candidates with claimed slots per relay parent. We need this information in order to remove
	/// claims. The key is the relay parent of the candidate and the value - the set of candidates
	/// at it.
	///
	/// Note 1: We can't map the candidates to an exact slot since we need to keep track on their
	/// ordering which will be an overkill in the context of `ClaimQueueState`. That's why we keep
	/// the claims on first came first claimed basis.
	///
	/// Let's say there are three candidates built on top of each other A->B->C. We claim slots for
	/// them in the order we receive them. If we receive them out of order the claims will be out
	/// of order too, but this is fine since we only care about the count. If we fetch B and C
	/// successfully and can't fetch A the user of `ClaimQueueState` must make sure that B and C
	/// are removed.
	///
	/// Note 2: During pruning we remove all the candidates for the pruned relay parent because we
	/// no longer need to know about them. If the claim was not undone so far - it will be
	/// permanent.
	candidates: HashMap<Hash, HashSet<Hash>>,
}

impl ClaimQueueState {
	/// Create an empty `ClaimQueueState`. Use [`add_leaf`] to populate it.
	pub fn new() -> Self {
		Self {
			block_state: VecDeque::new(),
			future_blocks: VecDeque::new(),
			candidates: HashMap::new(),
		}
	}

	/// Appends a new leaf with its corresponding claim queue to the state.
	pub fn add_leaf(&mut self, hash: &Hash, claim_queue: &VecDeque<ParaId>) {
		if self.block_state.iter().any(|s| s.hash == Some(*hash)) {
			return
		}

		// First check if our view for the future blocks is consistent with the one in the claim
		// queue of the new block. If not - the claim queue has changed for some reason and we need
		// to readjust our view.
		for (idx, expected_claim) in claim_queue.iter().enumerate() {
			match self.future_blocks.get_mut(idx) {
				Some(future_block) =>
					if future_block.claim.as_ref() != Some(expected_claim) {
						// There is an inconsistency. Update our view with the one from the claim
						// queue. `claimed` can't be true anymore since the `ParaId` has changed.
						future_block.claimed = ClaimState::Free;
						future_block.claim = Some(*expected_claim);

						// IMPORTANT: at this point there will be a slight inconsistency between
						// `block_state`/`future_blocks` and `candidates`. We just removed a future
						// claim but we can't be sure which candidate should be removed since we
						// don't know the exact ordering between them. So we just keep `candidates`
						// untouched which means we will sheepishly pretend there are claims for the
						// extra candidate. This is not ideal but the case is very rare and is not
						// worth the extra complexity for handling it.
					},
				None => {
					self.future_blocks.push_back(ClaimInfo {
						hash: None,
						claim: Some(*expected_claim),
						// For future blocks we don't know the size of the claim queue.
						// `claim_queue_len` could be an option but there is not much benefit from
						// the extra boilerplate code to handle it. We set it to one since we
						// usually know about one claim at each future block but this value is not
						// used anywhere in the code.
						claim_queue_len: 1,
						claimed: ClaimState::Free,
					});
				},
			}
		}

		// Now pop the first future block and add it as a leaf
		let claim_info = if let Some(new_leaf) = self.future_blocks.pop_front() {
			ClaimInfo {
				hash: Some(*hash),
				claim: claim_queue.front().copied(),
				claim_queue_len: claim_queue.len(),
				claimed: new_leaf.claimed,
			}
		} else {
			// maybe the claim queue was empty but we still need to add a leaf
			ClaimInfo {
				hash: Some(*hash),
				claim: claim_queue.front().copied(),
				claim_queue_len: claim_queue.len(),
				claimed: ClaimState::Free,
			}
		};

		// `future_blocks` can't be longer than the length of the claim queue at the last block - 1.
		// For example this can happen if at relay block N we have got a claim queue of a length 4
		// and it's shrunk to 2.
		self.future_blocks.truncate(claim_queue.len().saturating_sub(1));

		self.block_state.push_back(claim_info);
	}

	/// Claims the first available slot for `para_id` at `relay_parent`. Returns `true` if the claim
	/// was successful.
	pub fn claim_pending_at(
		&mut self,
		relay_parent: &Hash,
		para_id: &ParaId,
		maybe_candidate_hash: Option<Hash>,
	) -> bool {
		gum::trace!(
			target: LOG_TARGET,
			?para_id,
			?relay_parent,
			"claim_at"
		);

		self.find_a_free_claim(
			relay_parent,
			para_id,
			maybe_candidate_hash,
			ClaimState::Pending(maybe_candidate_hash),
		)
	}

	/// Looks for a pending claim for `candidate_hash` at `relay_parent` and upgrades it to
	/// seconded.
	pub fn upgrade_pending_claims(&mut self, relay_parent: &Hash, candidate_hash: &Hash) -> bool {
		gum::trace!(
			target: LOG_TARGET,
			?relay_parent,
			?candidate_hash,
			"upgrade_pending_claims"
		);

		if !self.candidates.contains_key(candidate_hash) {
			// Unknown candidate, no need to look in state.
			return false;
		}

		for w in self.get_window(relay_parent) {
			if w.claimed == ClaimState::Pending(Some(*candidate_hash)) {
				w.claimed = ClaimState::Seconded(*candidate_hash);
				return true;
			}
		}

		return false;
	}

	/// If there is a pending claim for the candidate at `relay_parent` it is upgraded to seconded.
	/// Otherwise a new claim is made.
	pub fn claim_seconded_at(
		&mut self,
		relay_parent: &Hash,
		para_id: &ParaId,
		candidate_hash: Hash,
	) -> bool {
		gum::trace!(
			target: LOG_TARGET,
			?para_id,
			?relay_parent,
			"second_at"
		);

		if self.candidates.get(relay_parent).map_or(false, |c| c.contains(&candidate_hash)) {
			let window = self.get_window(relay_parent);

			for w in window {
				if w.claimed == ClaimState::Pending(Some(candidate_hash)) {
					w.claimed = ClaimState::Seconded(candidate_hash);
					return true;
				}
			}

			gum::warn!(
				target: LOG_TARGET,
				?para_id,
				?relay_parent,
				?candidate_hash,
				"Hash found in candidates for can't find a claim for it. This should never happen"
			);

			return false
		} else {
			// this is a new claim
			self.find_a_free_claim(
				relay_parent,
				para_id,
				Some(candidate_hash),
				ClaimState::Seconded(candidate_hash),
			)
		}
	}

	/// Returns `true` if a claim can be made for `para_id` at `relay_parent`. The function only
	/// performs a check. No actual claim is made.
	pub fn can_claim_at(
		&mut self,
		relay_parent: &Hash,
		para_id: &ParaId,
		maybe_candidate_hash: Option<Hash>,
	) -> bool {
		gum::trace!(
			target: LOG_TARGET,
			?para_id,
			?relay_parent,
			"can_claim_at"
		);

		self.find_a_free_claim(relay_parent, para_id, maybe_candidate_hash, ClaimState::Free)
	}

	/// Returns a `Vec` of `ParaId`s with all pending claims `relay_parent`.
	pub fn claimed_at(&mut self, relay_parent: &Hash) -> VecDeque<ParaId> {
		let window = self.get_window(relay_parent);

		window
			.filter(|b| matches!(b.claimed, ClaimState::Pending(_)))
			.filter_map(|b| b.claim)
			.collect()
	}

	/// Removes pruned blocks from all paths. `targets` is a set of hashes which were pruned.
	/// `removed` should contain hashes in the beginning of the path otherwise they won't be
	/// removed.
	///
	/// Example: if a path is [A, B, C, D] and `removed` contains [A, B] then both A and B will be
	/// removed. But if `target` contains [B, C] then nothing will be removed.
	pub fn remove_pruned_ancestors(&mut self, targets: &HashSet<Hash>) {
		// First remove all entries from candidates for each removed relay parent. Any claimed
		// entries for it can't be undone anymore.
		for removed in targets {
			self.candidates.remove(removed);
		}

		// All the blocks that should be pruned are in the front of `block_state`. Since `target` is
		// not ordered - keep popping until the first element is not found in `targets`.
		loop {
			match self.block_state.front().and_then(|b| b.hash) {
				Some(h) if targets.contains(&h) => {
					self.block_state.pop_front();
				},
				_ => break,
			}
		}
	}

	/// Returns true if the path is empty
	pub fn empty(&self) -> bool {
		self.block_state.is_empty()
	}

	/// Removes a claim for a candidate
	pub fn remove_claim(&mut self, candidate_hash: &Hash) -> bool {
		// Get information about the claim (relay parent and para id) from candidates.
		let mut maybe_relay_parent = None;
		for (rp, candidates) in &mut self.candidates {
			if candidates.remove(candidate_hash) {
				maybe_relay_parent = Some(*rp);
				break
			}
		}

		let relay_parent = if let Some(rp) = maybe_relay_parent { rp } else { return false };

		// Release the last possible claim
		// Collect the iterator first because peekable iters can't be reversed
		let window = self.get_window(&relay_parent);
		for w in window {
			if w.claimed == ClaimState::Pending(Some(*candidate_hash)) ||
				w.claimed == ClaimState::Seconded(*candidate_hash)
			{
				w.claimed = ClaimState::Free;
				return true
			}
		}

		false
	}

	/// Returns a iterator over the claim queue of `relay_parent`
	fn get_window<'a>(
		&'a mut self,
		relay_parent: &'a Hash,
	) -> impl Iterator<Item = &mut ClaimInfo> + 'a {
		let mut window = self
			.block_state
			.iter_mut()
			.skip_while(|b| b.hash != Some(*relay_parent))
			.peekable();
		let cq_len = window.peek().map_or(0, |b| b.claim_queue_len);
		window.chain(self.future_blocks.iter_mut()).take(cq_len)
	}

	// Returns `true` if there is a claim within `relay_parent`'s view of the claim queue for
	// `para_id`. If `claim_it` is set to `true` the slot is claimed. Otherwise the function just
	// reports the availability of the slot.
	fn find_a_free_claim(
		&mut self,
		relay_parent: &Hash,
		para_id: &ParaId,
		maybe_candidate_hash: Option<Hash>,
		new_state: ClaimState,
	) -> bool {
		if let Some(candidate_hash) = maybe_candidate_hash {
			if self.candidates.get(relay_parent).map_or(false, |c| c.contains(&candidate_hash)) {
				// there is already a claim for this candidate - return now
				gum::trace!(
					target: LOG_TARGET,
					?para_id,
					?relay_parent,
					?candidate_hash,
					"Claim already exists"
				);
				return true
			}
		}

		let window = self.get_window(relay_parent);
		let make_a_claim = new_state != ClaimState::Free;

		let mut claim_found = false;
		for w in window {
			gum::trace!(
				target: LOG_TARGET,
				?para_id,
				?relay_parent,
				claim_info=?w,
				?new_state,
				"Checking claim"
			);

			if w.claimed == ClaimState::Free && w.claim == Some(*para_id) {
				w.claimed = new_state;

				claim_found = true;
				break
			}
		}

		// Save the candidate hash
		if let Some(candidate_hash) = maybe_candidate_hash {
			if claim_found && make_a_claim {
				self.candidates
					.entry(*relay_parent)
					.or_insert_with(HashSet::new)
					.insert(candidate_hash);
			}
		}

		claim_found
	}
}

/// Keeps a per leaf state of the claim queue for multiple forks.
pub struct PerLeafClaimQueueState {
	/// The state of the claim queue per leaf
	state: HashMap<Hash, ClaimQueueState>,
}

impl PerLeafClaimQueueState {
	/// Creates an empty `PerLeafClaimQueueState`
	pub fn new() -> Self {
		Self { state: HashMap::new() }
	}

	/// Adds new leaf to the state. If the parent of the leaf is already in the state `leaf` is
	/// added to the corresponding path. Otherwise a new path is created.
	pub fn add_leaf(&mut self, leaf: &Hash, claim_queue: &VecDeque<ParaId>, parent: &Hash) {
		let maybe_path = self.state.remove(parent);

		match maybe_path {
			Some(mut path) => {
				path.add_leaf(leaf, claim_queue);
				self.state.insert(*leaf, path);
				return
			},
			None => {
				// parent not found in state - add a new path
				let mut new_path_state = ClaimQueueState::new();
				new_path_state.add_leaf(leaf, claim_queue);
				self.state.insert(*leaf, new_path_state);
			},
		}
	}

	/// Removes a set of pruned blocks from all paths. If a path becomes empty it is removed from
	/// the state.
	pub fn remove_pruned_ancestors(&mut self, removed: &HashSet<Hash>) {
		// Remove the pruned blocks from the paths
		for (_, path) in &mut self.state {
			path.remove_pruned_ancestors(removed);
		}

		// Remove all empty paths
		self.state.retain(|_, p| !p.empty());
	}

	/// Returns `true` if there is a free claim within `relay_parent`'s view of the claim queue for
	/// `leaf` or if there already is a claimed slot for the candidate.
	pub fn has_free_slot_at_leaf_for(
		&mut self,
		leaf: &Hash,
		relay_parent: &Hash,
		para_id: &ParaId,
		candidate_hash: &Hash,
	) -> bool {
		self.state.get_mut(leaf).map_or(false, |p: &mut ClaimQueueState| {
			p.can_claim_at(relay_parent, para_id, Some(*candidate_hash))
		})
	}

	/// Returns `true` if there is a free claim for `para_id` at `relay_parent` in at least one
	/// leaf.
	pub fn has_free_slot_for_para_id(&mut self, relay_parent: &Hash, para_id: &ParaId) -> bool {
		for (_, state) in &mut self.state {
			if state.can_claim_at(relay_parent, para_id, None) {
				return true
			}
		}
		false
	}

	/// Releases a claim for a candidate.
	pub fn release_claims_for_candidate(&mut self, candidate_hash: &Hash) -> bool {
		let mut result = false;
		for (_, state) in &mut self.state {
			if state.remove_claim(candidate_hash) {
				result = true;
			}
		}
		result
	}

	/// Claims a slot in pending state for a candidate at each leaf. Returns true if the claim was
	/// successful for at least one leaf or there already is a claim for the candidate.
	pub fn claim_pending_slot(
		&mut self,
		candidate_hash: &Hash,
		relay_parent: &Hash,
		para_id: &ParaId,
	) -> bool {
		let mut result = false;
		for (_, state) in &mut self.state {
			if state.claim_pending_at(relay_parent, para_id, Some(*candidate_hash)) {
				result = true;
			}
		}
		result
	}

	/// Claims a slot in pending state for a candidate at a concrete leaf
	pub fn claim_pending_slot_at_leaf(
		&mut self,
		leaf: &Hash,
		candidate_hash: &Hash,
		relay_parent: &Hash,
		para_id: &ParaId,
	) -> bool {
		if let Some(leaf_state) = self.state.get_mut(leaf) {
			leaf_state.claim_pending_at(relay_parent, para_id, Some(*candidate_hash));
			return true
		}
		return false
	}

	/// Seconds a slot for a candidate at each leaf. Returns true if the claim was successful for at
	/// least one leaf.
	pub fn claim_seconded_slot(
		&mut self,
		candidate_hash: &Hash,
		relay_parent: &Hash,
		para_id: &ParaId,
	) -> bool {
		let mut result = false;
		for (_, state) in &mut self.state {
			if state.claim_seconded_at(relay_parent, para_id, *candidate_hash) {
				result = true;
			}
		}
		result
	}

	/// Upgrade the pending claims for `candidate_hash` at each leaf to seconded
	pub fn upgrade_pending_claims_to_seconded(
		&mut self,
		candidate_hash: &Hash,
		relay_parent: &Hash,
	) -> bool {
		let mut result = false;
		for (_, state) in &mut self.state {
			if state.upgrade_pending_claims(relay_parent, candidate_hash) {
				result = true;
			}
		}
		result
	}
	/// Returns claimed slots at a relay parent. As there can be multiple forks per relay parent,
	/// the longest one is returned.
	pub fn get_claimed_slots_at(&mut self, relay_parent: &Hash) -> VecDeque<ParaId> {
		self.state
			.iter_mut()
			.map(|(_, s)| s.claimed_at(relay_parent))
			.max_by(|a, b| a.len().cmp(&b.len()))
			.unwrap_or_default()
	}
}

#[cfg(test)]
mod test {
	use super::*;

	mod claim_queue_state {
		use super::*;

		#[test]
		fn sane_initial_state() {
			let mut state = ClaimQueueState::new();
			let relay_parent = Hash::from_low_u64_be(1);
			let para_id = ParaId::new(1);

			assert!(!state.can_claim_at(
				&relay_parent,
				&para_id,
				Some(&Hash::from_low_u64_be(101))
			));
			assert!(!state.claim_pending_at(
				&relay_parent,
				&para_id,
				Some(&Hash::from_low_u64_be(101))
			));
			assert_eq!(state.claimed_at(&relay_parent), vec![]);
		}

		#[test]
		fn add_leaf_works() {
			let mut state = ClaimQueueState::new();
			let relay_parent_a = Hash::from_low_u64_be(1);
			let para_id = ParaId::new(1);
			let claim_queue = VecDeque::from(vec![para_id, para_id, para_id]);

			state.add_leaf(&relay_parent_a, &claim_queue);
			assert_eq!(state.claimed_at(&relay_parent_a), vec![para_id, para_id, para_id]);

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![ClaimInfo {
					hash: Some(relay_parent_a),
					claim: Some(para_id),
					claim_queue_len: 3,
					claimed: false,
				},])
			);
			assert_eq!(
				state.future_blocks,
				VecDeque::from(vec![
					ClaimInfo {
						hash: None,
						claim: Some(para_id),
						claim_queue_len: 1,
						claimed: false
					},
					ClaimInfo {
						hash: None,
						claim: Some(para_id),
						claim_queue_len: 1,
						claimed: false
					}
				])
			);
			assert!(state.candidates.is_empty());

			// should be no op
			state.add_leaf(&relay_parent_a, &claim_queue);
			assert_eq!(state.block_state.len(), 1);
			assert_eq!(state.future_blocks.len(), 2);

			// add another leaf
			let relay_parent_b = Hash::from_low_u64_be(2);
			state.add_leaf(&relay_parent_b, &claim_queue);

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![
					ClaimInfo {
						hash: Some(relay_parent_a),
						claim: Some(para_id),
						claim_queue_len: 3,
						claimed: false,
					},
					ClaimInfo {
						hash: Some(relay_parent_b),
						claim: Some(para_id),
						claim_queue_len: 3,
						claimed: false,
					}
				])
			);
			assert_eq!(
				state.future_blocks,
				VecDeque::from(vec![
					ClaimInfo {
						hash: None,
						claim: Some(para_id),
						claim_queue_len: 1,
						claimed: false
					},
					ClaimInfo {
						hash: None,
						claim: Some(para_id),
						claim_queue_len: 1,
						claimed: false
					}
				])
			);
			assert!(state.candidates.is_empty());

			assert_eq!(state.claimed_at(&relay_parent_a), vec![para_id, para_id, para_id]);
			assert_eq!(state.claimed_at(&relay_parent_b), vec![para_id, para_id, para_id]);
		}

		#[test]
		fn claims_at_separate_relay_parents_work() {
			let mut state = ClaimQueueState::new();
			let relay_parent_a = Hash::from_low_u64_be(1);
			let relay_parent_b = Hash::from_low_u64_be(2);
			let para_id = ParaId::new(1);
			let claim_queue = VecDeque::from(vec![para_id, para_id, para_id]);

			state.add_leaf(&relay_parent_a, &claim_queue);
			state.add_leaf(&relay_parent_b, &claim_queue);

			// add one claim for a
			let candidate_a = Hash::from_low_u64_be(101);
			assert!(state.can_claim_at(&relay_parent_a, &para_id, Some(&candidate_a)));
			assert_eq!(state.claimed_at(&relay_parent_a), vec![para_id, para_id, para_id]);
			assert!(state.claim_pending_at(&relay_parent_a, &para_id, Some(&candidate_a)));

			// and one for b
			let candidate_b = Hash::from_low_u64_be(200);
			assert!(state.can_claim_at(&relay_parent_b, &para_id, Some(&candidate_b)));
			assert_eq!(state.claimed_at(&relay_parent_b), vec![para_id, para_id, para_id]);
			assert!(state.claim_pending_at(&relay_parent_b, &para_id, Some(&candidate_b)));

			// a should have one claim since the one for b was claimed
			assert_eq!(state.claimed_at(&relay_parent_a), vec![para_id]);
			// and two more for b
			assert_eq!(state.claimed_at(&relay_parent_b), vec![para_id, para_id]);

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![
					ClaimInfo {
						hash: Some(relay_parent_a),
						claim: Some(para_id),
						claim_queue_len: 3,
						claimed: true,
					},
					ClaimInfo {
						hash: Some(relay_parent_b),
						claim: Some(para_id),
						claim_queue_len: 3,
						claimed: true,
					}
				])
			);
			assert_eq!(
				state.future_blocks,
				VecDeque::from(vec![
					ClaimInfo {
						hash: None,
						claim: Some(para_id),
						claim_queue_len: 1,
						claimed: false
					},
					ClaimInfo {
						hash: None,
						claim: Some(para_id),
						claim_queue_len: 1,
						claimed: false
					}
				])
			);
			assert_eq!(
				state.candidates,
				HashMap::from_iter([
					(relay_parent_a, HashMap::from_iter(vec![(candidate_a, para_id)])),
					(relay_parent_b, HashMap::from_iter(vec![(candidate_b, para_id)]))
				])
			);
		}

		#[test]
		fn claims_are_transferred_to_next_slot() {
			let mut state = ClaimQueueState::new();
			let relay_parent_a = Hash::from_low_u64_be(1);
			let para_id = ParaId::new(1);
			let claim_queue = VecDeque::from(vec![para_id, para_id, para_id]);

			state.add_leaf(&relay_parent_a, &claim_queue);

			// add two claims, 2nd should be transferred to a new leaf
			let candidate_a1 = Hash::from_low_u64_be(101);
			assert!(state.can_claim_at(&relay_parent_a, &para_id, Some(&candidate_a1)));
			assert_eq!(state.claimed_at(&relay_parent_a), vec![para_id, para_id, para_id]);
			assert!(state.claim_pending_at(&relay_parent_a, &para_id, Some(&candidate_a1)));

			let candidate_a2 = Hash::from_low_u64_be(102);
			assert!(state.can_claim_at(&relay_parent_a, &para_id, Some(&candidate_a2)));
			assert_eq!(state.claimed_at(&relay_parent_a), vec![para_id, para_id]);
			assert!(state.claim_pending_at(&relay_parent_a, &para_id, Some(&candidate_a2)));

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![ClaimInfo {
					hash: Some(relay_parent_a),
					claim: Some(para_id),
					claim_queue_len: 3,
					claimed: true,
				},])
			);
			assert_eq!(
				state.future_blocks,
				VecDeque::from(vec![
					ClaimInfo {
						hash: None,
						claim: Some(para_id),
						claim_queue_len: 1,
						claimed: true
					},
					ClaimInfo {
						hash: None,
						claim: Some(para_id),
						claim_queue_len: 1,
						claimed: false
					}
				])
			);
			assert_eq!(
				state.candidates,
				HashMap::from_iter([(
					relay_parent_a,
					HashMap::from_iter(vec![(candidate_a1, para_id), (candidate_a2, para_id)])
				)])
			);

			// one more
			let candidate_a3 = Hash::from_low_u64_be(103);
			assert!(state.can_claim_at(&relay_parent_a, &para_id, Some(&candidate_a3)));
			assert_eq!(state.claimed_at(&relay_parent_a), vec![para_id]);
			assert!(state.claim_pending_at(&relay_parent_a, &para_id, Some(&candidate_a3)));

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![ClaimInfo {
					hash: Some(relay_parent_a),
					claim: Some(para_id),
					claim_queue_len: 3,
					claimed: true,
				},])
			);
			assert_eq!(
				state.future_blocks,
				VecDeque::from(vec![
					ClaimInfo {
						hash: None,
						claim: Some(para_id),
						claim_queue_len: 1,
						claimed: true
					},
					ClaimInfo {
						hash: None,
						claim: Some(para_id),
						claim_queue_len: 1,
						claimed: true
					}
				])
			);
			assert_eq!(
				state.candidates,
				HashMap::from_iter([(
					relay_parent_a,
					HashMap::from_iter(vec![
						(candidate_a1, para_id),
						(candidate_a2, para_id),
						(candidate_a3, para_id)
					])
				)])
			);

			// no more claims
			let candidate_a4 = Hash::from_low_u64_be(104);

			assert!(!state.can_claim_at(&relay_parent_a, &para_id, Some(&candidate_a4)));
			assert_eq!(state.claimed_at(&relay_parent_a), vec![]);
		}

		#[test]
		fn claims_are_transferred_to_new_leaves() {
			let mut state = ClaimQueueState::new();
			let relay_parent_a = Hash::from_low_u64_be(1);
			let para_id = ParaId::new(1);
			let claim_queue = VecDeque::from(vec![para_id, para_id, para_id]);

			state.add_leaf(&relay_parent_a, &claim_queue);

			let mut candidates = vec![];
			for i in 0..3 {
				candidates.push(Hash::from_low_u64_be(101 + i));
			}

			for c in &candidates {
				assert!(state.can_claim_at(&relay_parent_a, &para_id, Some(c)));
				assert!(state.claim_pending_at(&relay_parent_a, &para_id, Some(c)));
			}

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![ClaimInfo {
					hash: Some(relay_parent_a),
					claim: Some(para_id),
					claim_queue_len: 3,
					claimed: true,
				},])
			);
			assert_eq!(
				state.future_blocks,
				VecDeque::from(vec![
					ClaimInfo {
						hash: None,
						claim: Some(para_id),
						claim_queue_len: 1,
						claimed: true
					},
					ClaimInfo {
						hash: None,
						claim: Some(para_id),
						claim_queue_len: 1,
						claimed: true
					}
				])
			);
			assert_eq!(
				state.candidates,
				HashMap::from_iter([(
					relay_parent_a,
					HashMap::from_iter(
						candidates.iter().cloned().map(|c| (c, para_id)).collect::<Vec<_>>()
					)
				)])
			);

			// no more claims
			let new_candidate = Hash::from_low_u64_be(101 + candidates.len() as u64);
			assert!(!state.can_claim_at(&relay_parent_a, &para_id, Some(&new_candidate)));

			// new leaf
			let relay_parent_b = Hash::from_low_u64_be(2);
			state.add_leaf(&relay_parent_b, &claim_queue);

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![
					ClaimInfo {
						hash: Some(relay_parent_a),
						claim: Some(para_id),
						claim_queue_len: 3,
						claimed: true,
					},
					ClaimInfo {
						hash: Some(relay_parent_b),
						claim: Some(para_id),
						claim_queue_len: 3,
						claimed: true,
					}
				])
			);
			assert_eq!(
				state.future_blocks,
				VecDeque::from(vec![
					ClaimInfo {
						hash: None,
						claim: Some(para_id),
						claim_queue_len: 1,
						claimed: true
					},
					ClaimInfo {
						hash: None,
						claim: Some(para_id),
						claim_queue_len: 1,
						claimed: false
					}
				])
			);

			// still no claims for a
			assert!(!state.can_claim_at(&relay_parent_a, &para_id, Some(&new_candidate)));

			// but can accept for b
			assert!(state.can_claim_at(&relay_parent_b, &para_id, Some(&new_candidate)));
			assert!(state.claim_pending_at(&relay_parent_b, &para_id, Some(&new_candidate)));

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![
					ClaimInfo {
						hash: Some(relay_parent_a),
						claim: Some(para_id),
						claim_queue_len: 3,
						claimed: true,
					},
					ClaimInfo {
						hash: Some(relay_parent_b),
						claim: Some(para_id),
						claim_queue_len: 3,
						claimed: true,
					}
				])
			);
			assert_eq!(
				state.future_blocks,
				VecDeque::from(vec![
					ClaimInfo {
						hash: None,
						claim: Some(para_id),
						claim_queue_len: 1,
						claimed: true
					},
					ClaimInfo {
						hash: None,
						claim: Some(para_id),
						claim_queue_len: 1,
						claimed: true
					}
				])
			);
			assert_eq!(
				state.candidates,
				HashMap::from_iter([
					(
						relay_parent_a,
						HashMap::from_iter(
							candidates.iter().cloned().map(|c| (c, para_id)).collect::<Vec<_>>()
						)
					),
					(relay_parent_b, HashMap::from_iter(vec![(new_candidate, para_id)]))
				])
			);
		}

		#[test]
		fn two_paras() {
			let mut state = ClaimQueueState::new();
			let relay_parent_a = Hash::from_low_u64_be(1);
			let para_id_a = ParaId::new(1);
			let para_id_b = ParaId::new(2);
			let claim_queue = VecDeque::from(vec![para_id_a, para_id_b, para_id_a]);

			state.add_leaf(&relay_parent_a, &claim_queue);
			let candidate_a1 = Hash::from_low_u64_be(101);
			let candidate_b1 = Hash::from_low_u64_be(200);
			assert!(state.can_claim_at(&relay_parent_a, &para_id_a, Some(&candidate_a1)));
			assert!(state.can_claim_at(&relay_parent_a, &para_id_b, Some(&candidate_b1)));
			assert_eq!(state.claimed_at(&relay_parent_a), vec![para_id_a, para_id_b, para_id_a]);

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![ClaimInfo {
					hash: Some(relay_parent_a),
					claim: Some(para_id_a),
					claim_queue_len: 3,
					claimed: false,
				},])
			);
			assert_eq!(
				state.future_blocks,
				VecDeque::from(vec![
					ClaimInfo {
						hash: None,
						claim: Some(para_id_b),
						claim_queue_len: 1,
						claimed: false
					},
					ClaimInfo {
						hash: None,
						claim: Some(para_id_a),
						claim_queue_len: 1,
						claimed: false
					}
				])
			);
			assert!(state.candidates.is_empty());

			// claim another candidate
			assert!(state.claim_pending_at(&relay_parent_a, &para_id_a, Some(&candidate_a1)));

			// we should still be able to claim candidates for both paras
			let candidate_a2 = Hash::from_low_u64_be(102);
			assert!(state.can_claim_at(&relay_parent_a, &para_id_a, Some(&candidate_a2)));
			assert!(state.can_claim_at(&relay_parent_a, &para_id_b, Some(&candidate_b1)));
			assert_eq!(state.claimed_at(&relay_parent_a), vec![para_id_b, para_id_a]);

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![ClaimInfo {
					hash: Some(relay_parent_a),
					claim: Some(para_id_a),
					claim_queue_len: 3,
					claimed: true,
				},])
			);
			assert_eq!(
				state.future_blocks,
				VecDeque::from(vec![
					ClaimInfo {
						hash: None,
						claim: Some(para_id_b),
						claim_queue_len: 1,
						claimed: false
					},
					ClaimInfo {
						hash: None,
						claim: Some(para_id_a),
						claim_queue_len: 1,
						claimed: false
					}
				])
			);
			assert_eq!(
				state.candidates,
				HashMap::from_iter([(
					relay_parent_a,
					HashMap::from_iter(vec![(candidate_a1, para_id_a)])
				),])
			);

			// another claim for a
			assert!(state.claim_pending_at(&relay_parent_a, &para_id_a, Some(&candidate_a2)));

			// no more claims for a, but should be able to claim for b
			let candidate_a3 = Hash::from_low_u64_be(103);
			assert!(!state.can_claim_at(&relay_parent_a, &para_id_a, Some(&candidate_a3)));
			assert!(state.can_claim_at(&relay_parent_a, &para_id_b, Some(&candidate_b1)));
			assert_eq!(state.claimed_at(&relay_parent_a), vec![para_id_b]);

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![ClaimInfo {
					hash: Some(relay_parent_a),
					claim: Some(para_id_a),
					claim_queue_len: 3,
					claimed: true,
				},])
			);
			assert_eq!(
				state.future_blocks,
				VecDeque::from(vec![
					ClaimInfo {
						hash: None,
						claim: Some(para_id_b),
						claim_queue_len: 1,
						claimed: false
					},
					ClaimInfo {
						hash: None,
						claim: Some(para_id_a),
						claim_queue_len: 1,
						claimed: true
					}
				])
			);
			assert_eq!(
				state.candidates,
				HashMap::from_iter([(
					relay_parent_a,
					HashMap::from_iter(vec![(candidate_a1, para_id_a), (candidate_a2, para_id_a)])
				)])
			);

			// another claim for b
			assert!(state.claim_pending_at(&relay_parent_a, &para_id_b, Some(&candidate_b1)));

			// no more claims neither for a nor for b
			let candidate_b2 = Hash::from_low_u64_be(201);
			assert!(!state.can_claim_at(&relay_parent_a, &para_id_a, Some(&candidate_a3)));
			assert!(!state.can_claim_at(&relay_parent_a, &para_id_b, Some(&candidate_b2)));
			assert_eq!(state.claimed_at(&relay_parent_a), vec![]);

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![ClaimInfo {
					hash: Some(relay_parent_a),
					claim: Some(para_id_a),
					claim_queue_len: 3,
					claimed: true,
				},])
			);
			assert_eq!(
				state.future_blocks,
				VecDeque::from(vec![
					ClaimInfo {
						hash: None,
						claim: Some(para_id_b),
						claim_queue_len: 1,
						claimed: true
					},
					ClaimInfo {
						hash: None,
						claim: Some(para_id_a),
						claim_queue_len: 1,
						claimed: true
					}
				])
			);
			assert_eq!(
				state.candidates,
				HashMap::from_iter([(
					relay_parent_a,
					HashMap::from_iter(vec![
						(candidate_a1, para_id_a),
						(candidate_a2, para_id_a),
						(candidate_b1, para_id_b)
					])
				)])
			);
		}

		#[test]
		fn claim_queue_changes_unexpectedly() {
			let mut state = ClaimQueueState::new();
			let relay_parent_a = Hash::from_low_u64_be(1);
			let para_id_a = ParaId::new(1);
			let para_id_b = ParaId::new(2);
			let claim_queue_a = VecDeque::from(vec![para_id_a, para_id_b, para_id_a]);

			state.add_leaf(&relay_parent_a, &claim_queue_a);
			let candidate_a1 = Hash::from_low_u64_be(101);
			let candidate_a2 = Hash::from_low_u64_be(102);
			let candidate_b = Hash::from_low_u64_be(200);

			assert!(state.can_claim_at(&relay_parent_a, &para_id_a, Some(&candidate_a1)));
			assert!(state.can_claim_at(&relay_parent_a, &para_id_b, Some(&candidate_b)));
			assert!(state.claim_pending_at(&relay_parent_a, &para_id_a, Some(&candidate_a1)));
			assert!(state.claim_pending_at(&relay_parent_a, &para_id_a, Some(&candidate_a2)));
			assert!(state.claim_pending_at(&relay_parent_a, &para_id_b, Some(&candidate_b)));
			assert_eq!(state.claimed_at(&relay_parent_a), vec![]);

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![ClaimInfo {
					hash: Some(relay_parent_a),
					claim: Some(para_id_a),
					claim_queue_len: 3,
					claimed: true,
				},])
			);
			assert_eq!(
				state.future_blocks,
				VecDeque::from(vec![
					ClaimInfo {
						hash: None,
						claim: Some(para_id_b),
						claim_queue_len: 1,
						claimed: true
					},
					ClaimInfo {
						hash: None,
						claim: Some(para_id_a),
						claim_queue_len: 1,
						claimed: true
					}
				])
			);
			assert_eq!(
				state.candidates,
				HashMap::from_iter([(
					relay_parent_a,
					HashMap::from_iter(vec![
						(candidate_a1, para_id_a),
						(candidate_a2, para_id_a),
						(candidate_b, para_id_b)
					])
				)])
			);

			let relay_parent_b = Hash::from_low_u64_be(2);
			let claim_queue_b = VecDeque::from(vec![para_id_a, para_id_a, para_id_a]); // should be [b, a, ...]
			state.add_leaf(&relay_parent_b, &claim_queue_b);

			// because of the unexpected change in claim queue we lost the claim for paraB and have
			// one unclaimed for paraA
			assert_eq!(state.claimed_at(&relay_parent_a), vec![para_id_a]);

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![
					ClaimInfo {
						hash: Some(relay_parent_a),
						claim: Some(para_id_a),
						claim_queue_len: 3,
						claimed: true,
					},
					ClaimInfo {
						hash: Some(relay_parent_b),
						claim: Some(para_id_a),
						claim_queue_len: 3,
						claimed: false,
					}
				])
			);
			assert_eq!(
				state.future_blocks,
				// since the 3rd slot of the claim queue at rp1 is equal to the second one in rp2,
				// this claim still exists
				VecDeque::from(vec![
					ClaimInfo {
						hash: None,
						claim: Some(para_id_a),
						claim_queue_len: 1,
						claimed: true
					},
					ClaimInfo {
						hash: None,
						claim: Some(para_id_a),
						claim_queue_len: 1,
						claimed: false
					}
				])
			);
			// IMPORTANT: we don't change `candidates`
			assert_eq!(
				state.candidates,
				HashMap::from_iter([(
					relay_parent_a,
					HashMap::from_iter(vec![
						(candidate_a1, para_id_a),
						(candidate_a2, para_id_a),
						(candidate_b, para_id_b)
					])
				)])
			);
		}

		#[test]
		fn claim_queue_changes_unexpectedly_with_two_blocks() {
			let mut state = ClaimQueueState::new();
			let relay_parent_a = Hash::from_low_u64_be(1);
			let para_id_a = ParaId::new(1);
			let para_id_b = ParaId::new(2);
			let claim_queue_a = VecDeque::from(vec![para_id_a, para_id_b, para_id_b]);

			state.add_leaf(&relay_parent_a, &claim_queue_a);
			let candidate_a = Hash::from_low_u64_be(101);
			let candidate_b1 = Hash::from_low_u64_be(200);
			let candidate_b2 = Hash::from_low_u64_be(201);
			assert!(state.can_claim_at(&relay_parent_a, &para_id_a, Some(&candidate_a)));
			assert!(state.can_claim_at(&relay_parent_a, &para_id_b, Some(&candidate_b1)));
			assert!(state.claim_pending_at(&relay_parent_a, &para_id_a, Some(&candidate_a)));
			assert!(state.claim_pending_at(&relay_parent_a, &para_id_b, Some(&candidate_b1)));
			assert!(state.claim_pending_at(&relay_parent_a, &para_id_b, Some(&candidate_b2)));
			assert_eq!(state.claimed_at(&relay_parent_a), vec![]);

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![ClaimInfo {
					hash: Some(relay_parent_a),
					claim: Some(para_id_a),
					claim_queue_len: 3,
					claimed: true,
				},])
			);
			assert_eq!(
				state.future_blocks,
				VecDeque::from(vec![
					ClaimInfo {
						hash: None,
						claim: Some(para_id_b),
						claim_queue_len: 1,
						claimed: true
					},
					ClaimInfo {
						hash: None,
						claim: Some(para_id_b),
						claim_queue_len: 1,
						claimed: true
					}
				])
			);
			assert_eq!(
				state.candidates,
				HashMap::from_iter([(
					relay_parent_a,
					HashMap::from_iter(vec![
						(candidate_a, para_id_a),
						(candidate_b1, para_id_b),
						(candidate_b2, para_id_b)
					])
				)])
			);

			let relay_parent_b = Hash::from_low_u64_be(2);
			let claim_queue_b = VecDeque::from(vec![para_id_a, para_id_a, para_id_a]); // should be [b, b, ...]
			state.add_leaf(&relay_parent_b, &claim_queue_b);

			// because of the unexpected change in claim queue we lost both claims for paraB and
			// have two unclaimed for paraA
			assert_eq!(state.claimed_at(&relay_parent_a), vec![para_id_a, para_id_a]);

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![
					ClaimInfo {
						hash: Some(relay_parent_a),
						claim: Some(para_id_a),
						claim_queue_len: 3,
						claimed: true,
					},
					ClaimInfo {
						hash: Some(relay_parent_b),
						claim: Some(para_id_a),
						claim_queue_len: 3,
						claimed: false,
					}
				])
			);
			assert_eq!(
				state.future_blocks,
				VecDeque::from(vec![
					ClaimInfo {
						hash: None,
						claim: Some(para_id_a),
						claim_queue_len: 1,
						claimed: false
					},
					ClaimInfo {
						hash: None,
						claim: Some(para_id_a),
						claim_queue_len: 1,
						claimed: false
					}
				])
			);
			// IMPORTANT: we don't change `candidates`
			assert_eq!(
				state.candidates,
				HashMap::from_iter([(
					relay_parent_a,
					HashMap::from_iter(vec![
						(candidate_a, para_id_a),
						(candidate_b1, para_id_b),
						(candidate_b2, para_id_b)
					])
				)])
			);
		}

		#[test]
		fn basic_remove_works() {
			let mut state = ClaimQueueState::new();
			let relay_parent_a = Hash::from_low_u64_be(1);
			let relay_parent_b = Hash::from_low_u64_be(2);
			let para_id = ParaId::new(1);
			let claim_queue = VecDeque::from(vec![para_id, para_id, para_id]);

			state.add_leaf(&relay_parent_a, &claim_queue);
			state.add_leaf(&relay_parent_b, &claim_queue);

			// add one claim per leaf
			let candidate_a1 = Hash::from_low_u64_be(101);
			let candidate_a2 = Hash::from_low_u64_be(102);
			state.claim_pending_at(&relay_parent_a, &para_id, Some(&candidate_a1));
			state.claim_pending_at(&relay_parent_b, &para_id, Some(&candidate_a2));

			let removed = vec![relay_parent_a];
			state.remove_pruned_ancestors(&HashSet::from_iter(removed.iter().cloned()));

			assert_eq!(state.block_state.len(), 1);
			assert_eq!(state.block_state[0].hash, Some(relay_parent_b));
			assert_eq!(state.future_blocks.len(), 2);
			assert_eq!(
				state.candidates,
				HashMap::from_iter([(
					relay_parent_b,
					HashMap::from_iter(vec![(candidate_a2, para_id)])
				)])
			);
		}

		#[test]
		fn remove_non_first_does_nothing() {
			let mut state = ClaimQueueState::new();
			let relay_parent_a = Hash::from_low_u64_be(1);
			let relay_parent_b = Hash::from_low_u64_be(2);
			let para_id = ParaId::new(1);
			let claim_queue = VecDeque::from(vec![para_id, para_id, para_id]);

			state.add_leaf(&relay_parent_a, &claim_queue);
			state.add_leaf(&relay_parent_b, &claim_queue);

			let removed = vec![relay_parent_b];
			state.remove_pruned_ancestors(&HashSet::from_iter(removed.iter().cloned()));

			assert_eq!(state.block_state.len(), 2);
			assert_eq!(state.future_blocks.len(), 2);
		}

		#[test]
		fn remove_multiple_works() {
			let mut state = ClaimQueueState::new();
			let relay_parent_a = Hash::from_low_u64_be(1);
			let relay_parent_b = Hash::from_low_u64_be(2);
			let relay_parent_c = Hash::from_low_u64_be(3);
			let para_id = ParaId::new(1);
			let claim_queue = VecDeque::from(vec![para_id, para_id, para_id]);

			let candidate_a1 = Hash::from_low_u64_be(101);
			let candidate_a2 = Hash::from_low_u64_be(102);
			let candidate_a3 = Hash::from_low_u64_be(103);
			state.add_leaf(&relay_parent_a, &claim_queue);
			state.claim_pending_at(&relay_parent_a, &para_id, Some(&candidate_a1));
			state.add_leaf(&relay_parent_b, &claim_queue);
			state.claim_pending_at(&relay_parent_b, &para_id, Some(&candidate_a2));
			state.add_leaf(&relay_parent_c, &claim_queue);
			state.claim_pending_at(&relay_parent_c, &para_id, Some(&candidate_a3));

			let removed = vec![relay_parent_a, relay_parent_b];
			state.remove_pruned_ancestors(&HashSet::from_iter(removed.iter().cloned()));

			assert_eq!(state.block_state.len(), 1);
			assert_eq!(state.block_state[0].hash, Some(relay_parent_c));
			assert_eq!(state.future_blocks.len(), 2);
			assert_eq!(
				state.candidates,
				HashMap::from_iter([(
					relay_parent_c,
					HashMap::from_iter(vec![(candidate_a3, para_id)])
				)])
			);
		}

		#[test]
		fn empty_claim_queue() {
			let mut state = ClaimQueueState::new();
			let relay_parent_a = Hash::from_low_u64_be(1);
			let para_id_a = ParaId::new(1);
			let claim_queue_a = VecDeque::new();
			let candidate_a1 = Hash::from_low_u64_be(101);

			state.add_leaf(&relay_parent_a, &claim_queue_a);
			assert_eq!(state.claimed_at(&relay_parent_a), vec![]);

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![ClaimInfo {
					hash: Some(relay_parent_a),
					claim: None,
					claim_queue_len: 0,
					claimed: false,
				},])
			);
			// no claim queue so we know nothing about future blocks
			assert!(state.future_blocks.is_empty());

			assert!(!state.can_claim_at(&relay_parent_a, &para_id_a, Some(&candidate_a1)));
			assert!(!state.claim_pending_at(&relay_parent_a, &para_id_a, Some(&candidate_a1)));
			assert_eq!(state.claimed_at(&relay_parent_a), vec![]);

			let relay_parent_b = Hash::from_low_u64_be(2);
			let claim_queue_b = VecDeque::from(vec![para_id_a]);
			state.add_leaf(&relay_parent_b, &claim_queue_b);

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![
					ClaimInfo {
						hash: Some(relay_parent_a),
						claim: None,
						claim_queue_len: 0,
						claimed: false,
					},
					ClaimInfo {
						hash: Some(relay_parent_b),
						claim: Some(para_id_a),
						claim_queue_len: 1,
						claimed: false,
					},
				])
			);
			// claim queue with length 1 doesn't say anything about future blocks
			assert!(state.future_blocks.is_empty());

			let candidate_a2 = Hash::from_low_u64_be(102);
			assert!(!state.can_claim_at(&relay_parent_a, &para_id_a, Some(&candidate_a2)));
			assert!(!state.claim_pending_at(&relay_parent_a, &para_id_a, Some(&candidate_a2)));
			assert_eq!(state.claimed_at(&relay_parent_a), vec![]);

			assert!(state.can_claim_at(&relay_parent_b, &para_id_a, Some(&candidate_a2)));
			assert_eq!(state.claimed_at(&relay_parent_b), vec![para_id_a]);
			assert!(state.claim_pending_at(&relay_parent_b, &para_id_a, Some(&candidate_a2)));

			let relay_parent_c = Hash::from_low_u64_be(3);
			let claim_queue_c = VecDeque::from(vec![para_id_a, para_id_a]);
			state.add_leaf(&relay_parent_c, &claim_queue_c);

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![
					ClaimInfo {
						hash: Some(relay_parent_a),
						claim: None,
						claim_queue_len: 0,
						claimed: false,
					},
					ClaimInfo {
						hash: Some(relay_parent_b),
						claim: Some(para_id_a),
						claim_queue_len: 1,
						claimed: true,
					},
					ClaimInfo {
						hash: Some(relay_parent_c),
						claim: Some(para_id_a),
						claim_queue_len: 2,
						claimed: false,
					},
				])
			);
			// claim queue with length 2 fills only one future block
			assert_eq!(
				state.future_blocks,
				VecDeque::from(vec![ClaimInfo {
					hash: None,
					claim: Some(para_id_a),
					claim_queue_len: 1,
					claimed: false,
				},])
			);

			let candidate_a3 = Hash::from_low_u64_be(103);

			assert!(!state.can_claim_at(&relay_parent_a, &para_id_a, Some(&candidate_a3)));
			assert!(!state.claim_pending_at(&relay_parent_a, &para_id_a, Some(&candidate_a3)));
			assert_eq!(state.claimed_at(&relay_parent_a), vec![]);

			// already claimed
			assert!(!state.can_claim_at(&relay_parent_b, &para_id_a, Some(&candidate_a3)));
			assert_eq!(state.claimed_at(&relay_parent_b), vec![]);
			assert!(!state.claim_pending_at(&relay_parent_b, &para_id_a, Some(&candidate_a3)));

			assert!(state.can_claim_at(&relay_parent_c, &para_id_a, Some(&candidate_a3)));
			assert_eq!(state.claimed_at(&relay_parent_c), vec![para_id_a, para_id_a]);
		}

		#[test]
		fn claim_queue_becomes_shorter() {
			let mut state = ClaimQueueState::new();
			let relay_parent_a = Hash::from_low_u64_be(1);
			let para_id_a = ParaId::new(1);
			let para_id_b = ParaId::new(2);
			let claim_queue_a = VecDeque::from(vec![para_id_a, para_id_b, para_id_a]);

			state.add_leaf(&relay_parent_a, &claim_queue_a);
			assert_eq!(state.claimed_at(&relay_parent_a), vec![para_id_a, para_id_b, para_id_a]);

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![ClaimInfo {
					hash: Some(relay_parent_a),
					claim: Some(para_id_a),
					claim_queue_len: 3,
					claimed: false,
				},])
			);
			assert_eq!(
				state.future_blocks,
				VecDeque::from(vec![
					ClaimInfo {
						hash: None,
						claim: Some(para_id_b),
						claim_queue_len: 1,
						claimed: false
					},
					ClaimInfo {
						hash: None,
						claim: Some(para_id_a),
						claim_queue_len: 1,
						claimed: false
					}
				])
			);

			let relay_parent_b = Hash::from_low_u64_be(2);
			let claim_queue_b = VecDeque::from(vec![para_id_a, para_id_b]); // should be [b, a]
			state.add_leaf(&relay_parent_b, &claim_queue_b);

			assert_eq!(state.claimed_at(&relay_parent_b), vec![para_id_a, para_id_b]);
			// claims for `relay_parent_a` has changed.
			assert_eq!(state.claimed_at(&relay_parent_a), vec![para_id_a, para_id_a, para_id_b]);

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![
					ClaimInfo {
						hash: Some(relay_parent_a),
						claim: Some(para_id_a),
						claim_queue_len: 3,
						claimed: false,
					},
					ClaimInfo {
						hash: Some(relay_parent_b),
						claim: Some(para_id_a),
						claim_queue_len: 2,
						claimed: false,
					}
				])
			);
			assert_eq!(
				state.future_blocks,
				VecDeque::from(vec![ClaimInfo {
					hash: None,
					claim: Some(para_id_b),
					claim_queue_len: 1,
					claimed: false
				},])
			);
		}

		#[test]
		fn claim_queue_becomes_shorter_and_drops_future_claims() {
			let mut state = ClaimQueueState::new();
			let relay_parent_a = Hash::from_low_u64_be(1);
			let para_id_a = ParaId::new(1);
			let para_id_b = ParaId::new(2);
			let claim_queue_a = VecDeque::from(vec![para_id_a, para_id_b, para_id_a, para_id_b]);

			state.add_leaf(&relay_parent_a, &claim_queue_a);

			assert_eq!(
				state.claimed_at(&relay_parent_a),
				vec![para_id_a, para_id_b, para_id_a, para_id_b]
			);

			// We start with claim queue len 4.
			assert_eq!(
				state.block_state,
				VecDeque::from(vec![ClaimInfo {
					hash: Some(relay_parent_a),
					claim: Some(para_id_a),
					claim_queue_len: 4,
					claimed: false,
				},])
			);
			// we have got three future blocks
			assert_eq!(
				state.future_blocks,
				VecDeque::from(vec![
					ClaimInfo {
						hash: None,
						claim: Some(para_id_b),
						claim_queue_len: 1,
						claimed: false
					},
					ClaimInfo {
						hash: None,
						claim: Some(para_id_a),
						claim_queue_len: 1,
						claimed: false
					},
					ClaimInfo {
						hash: None,
						claim: Some(para_id_b),
						claim_queue_len: 1,
						claimed: false
					}
				])
			);

			// The next claim len is 2, so we loose one future block
			let relay_parent_b = Hash::from_low_u64_be(2);
			let para_id_a = ParaId::new(1);
			let para_id_b = ParaId::new(2);
			let claim_queue_b = VecDeque::from(vec![para_id_b, para_id_a]);
			state.add_leaf(&relay_parent_b, &claim_queue_b);

			assert_eq!(state.claimed_at(&relay_parent_a), vec![para_id_a, para_id_b, para_id_a]);
			assert_eq!(state.claimed_at(&relay_parent_b), vec![para_id_b, para_id_a]);

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![
					ClaimInfo {
						hash: Some(relay_parent_a),
						claim: Some(para_id_a),
						claim_queue_len: 4,
						claimed: false,
					},
					ClaimInfo {
						hash: Some(relay_parent_b),
						claim: Some(para_id_b),
						claim_queue_len: 2,
						claimed: false,
					}
				])
			);

			assert_eq!(
				state.future_blocks,
				VecDeque::from(vec![ClaimInfo {
					hash: None,
					claim: Some(para_id_a),
					claim_queue_len: 1,
					claimed: false
				},])
			);
		}

		#[test]
		fn remove_claim_works() {
			let mut state = ClaimQueueState::new();
			let relay_parent_a = Hash::from_low_u64_be(1);
			let para_id = ParaId::new(1);
			let claim_queue = VecDeque::from(vec![para_id, para_id, para_id]);

			state.add_leaf(&relay_parent_a, &claim_queue);

			let candidate_a1 = Hash::from_low_u64_be(101);
			let candidate_a2 = Hash::from_low_u64_be(102);
			state.claim_pending_at(&relay_parent_a, &para_id, Some(&candidate_a1));
			state.claim_pending_at(&relay_parent_a, &para_id, Some(&candidate_a2));

			assert_eq!(state.claimed_at(&relay_parent_a), vec![para_id]);

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![ClaimInfo {
					hash: Some(relay_parent_a),
					claim: Some(para_id),
					claim_queue_len: 3,
					claimed: true,
				},])
			);
			assert_eq!(
				state.future_blocks,
				VecDeque::from(vec![
					ClaimInfo {
						hash: None,
						claim: Some(para_id),
						claim_queue_len: 1,
						claimed: true
					},
					ClaimInfo {
						hash: None,
						claim: Some(para_id),
						claim_queue_len: 1,
						claimed: false
					}
				])
			);
			assert_eq!(
				state.candidates,
				HashMap::from_iter([(
					relay_parent_a,
					HashMap::from_iter(vec![(candidate_a1, para_id), (candidate_a2, para_id)])
				)])
			);

			state.remove_claim(&candidate_a1);

			assert_eq!(state.claimed_at(&relay_parent_a), vec![para_id, para_id]);

			assert_eq!(
				state.block_state,
				VecDeque::from(vec![ClaimInfo {
					hash: Some(relay_parent_a),
					claim: Some(para_id),
					claim_queue_len: 3,
					claimed: true,
				},])
			);
			assert_eq!(
				state.future_blocks,
				VecDeque::from(vec![
					ClaimInfo {
						hash: None,
						claim: Some(para_id),
						claim_queue_len: 1,
						claimed: false
					},
					ClaimInfo {
						hash: None,
						claim: Some(para_id),
						claim_queue_len: 1,
						claimed: false
					}
				])
			);
			assert_eq!(
				state.candidates,
				HashMap::from_iter([(
					relay_parent_a,
					HashMap::from_iter(vec![(candidate_a2, para_id)])
				)])
			);
		}
	}
	mod per_leaf_claim_queue_state {
		use super::*;

		#[test]
		fn add_leaf_works() {
			let mut state = PerLeafClaimQueueState::new();
			let para_id = ParaId::new(1);
			let claim_queue = VecDeque::from(vec![para_id, para_id, para_id]);
			let relay_parent_a = Hash::from_low_u64_be(1);
			let relay_parent_b = Hash::from_low_u64_be(2);
			let relay_parent_c = Hash::from_low_u64_be(3);

			// 0 -> a -> b
			//  \-> c
			state.add_leaf(&relay_parent_a, &claim_queue, &Hash::from_low_u64_be(0));
			assert_eq!(state.state.len(), 1);

			state.add_leaf(&relay_parent_b, &claim_queue, &relay_parent_a);
			assert_eq!(state.state.len(), 1);

			state.add_leaf(&relay_parent_c, &claim_queue, &Hash::from_low_u64_be(0));
			assert_eq!(state.state.len(), 2);
		}

		#[test]
		fn remove_works() {
			let mut state = PerLeafClaimQueueState::new();
			let para_id = ParaId::new(1);
			let claim_queue = VecDeque::from(vec![para_id, para_id, para_id]);
			let relay_parent_a = Hash::from_low_u64_be(1);
			let relay_parent_b = Hash::from_low_u64_be(2);
			let relay_parent_c = Hash::from_low_u64_be(3);

			// 0 -> a -> b
			//  \-> c
			state.add_leaf(&relay_parent_a, &claim_queue, &Hash::from_low_u64_be(0));
			state.add_leaf(&relay_parent_b, &claim_queue, &relay_parent_a);
			state.add_leaf(&relay_parent_c, &claim_queue, &Hash::from_low_u64_be(0));

			let removed = vec![relay_parent_a, relay_parent_b];
			state.remove_pruned_ancestors(&HashSet::from_iter(removed.iter().cloned()));

			assert_eq!(state.state.len(), 1);
			assert_eq!(state.state[&relay_parent_c].block_state.len(), 1);
		}

		#[test]
		fn has_slot_at_leaf_for_works() {
			let mut state = PerLeafClaimQueueState::new();
			let para_id = ParaId::new(1);
			let claim_queue = VecDeque::from(vec![para_id, para_id]);
			let relay_parent_a = Hash::from_low_u64_be(1);
			let candidate_a = Hash::from_low_u64_be(101);
			let candidate_b = Hash::from_low_u64_be(102);
			let candidate_c = Hash::from_low_u64_be(103);

			state.add_leaf(&relay_parent_a, &claim_queue, &Hash::from_low_u64_be(0));

			assert!(state.has_free_slot_at_leaf_for(
				&relay_parent_a,
				&relay_parent_a,
				&para_id,
				&candidate_a
			));

			assert!(state.has_free_slot_at_leaf_for(
				&relay_parent_a,
				&relay_parent_a,
				&para_id,
				&candidate_b
			));

			assert!(state.has_free_slot_at_leaf_for(
				&relay_parent_a,
				&relay_parent_a,
				&para_id,
				&candidate_c
			));

			assert!(state.claim_slot(&candidate_a, &relay_parent_a, &para_id));
			assert!(state.claim_slot(&candidate_b, &relay_parent_a, &para_id,));

			// Because `candidate_a` already has got a slot
			assert!(state.has_free_slot_at_leaf_for(
				&relay_parent_a,
				&relay_parent_a,
				&para_id,
				&candidate_a
			));

			// Because `candidate_b` already has got a slot
			assert!(state.has_free_slot_at_leaf_for(
				&relay_parent_a,
				&relay_parent_a,
				&para_id,
				&candidate_b
			));

			// Because there are no free slots
			assert!(!state.has_free_slot_at_leaf_for(
				&relay_parent_a,
				&relay_parent_a,
				&para_id,
				&candidate_c
			));
		}
	}
}
