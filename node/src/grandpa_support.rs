// Copyright 2017-2020 Parity Technologies (UK) Ltd.
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

//! Polkadot-specific GRANDPA integration utilities.

use primitives::Hash;
use sp_runtime::traits::{Block as BlockT, NumberFor};

/// A custom GRANDPA voting rule that "pauses" voting (i.e. keeps voting for the
/// same last finalized block) after a given block at height `N` has been
/// finalized and for a delay of `M` blocks, i.e. until the best block reaches
/// `N` + `M`, the voter will keep voting for block `N`.
pub(crate) struct PauseAfterBlockFor<N>(pub(crate) N, pub(crate) N);

impl<Block, B> sc_finality_grandpa::VotingRule<Block, B> for PauseAfterBlockFor<NumberFor<Block>> where
	Block: BlockT,
	B: sp_blockchain::HeaderBackend<Block>,
{
	fn restrict_vote(
		&self,
		backend: &B,
		base: &Block::Header,
		best_target: &Block::Header,
		current_target: &Block::Header,
	) -> Option<(Block::Hash, NumberFor<Block>)> {
		use sp_runtime::generic::BlockId;
		use sp_runtime::traits::Header as _;

		// walk backwards until we find the target block
		let find_target = |
			target_number: NumberFor<Block>,
			current_header: &Block::Header
		| {
			let mut target_hash = current_header.hash();
			let mut target_header = current_header.clone();

			loop {
				if *target_header.number() < target_number {
					unreachable!(
						"we are traversing backwards from a known block; \
						 blocks are stored contiguously; \
						 qed"
					);
				}

				if *target_header.number() == target_number {
					return Some((target_hash, target_number));
				}

				target_hash = *target_header.parent_hash();
				target_header = backend.header(BlockId::Hash(target_hash)).ok()?
					.expect("Header known to exist due to the existence of one of its descendents; qed");
			}
		};

		// only restrict votes targeting a block higher than the block
		// we've set for the pause
		if *current_target.number() > self.0 {
			// if we're past the pause period (i.e. `self.0 + self.1`)
			// then we no longer need to restrict any votes
			if *best_target.number() > self.0 + self.1 {
				return None;
			}

			// if we've finalized the pause block, just keep returning it
			// until best number increases enough to pass the condition above
			if *base.number() >= self.0 {
				return Some((base.hash(), *base.number()));
			}

			// otherwise find the target header at the pause block
			// to vote on
			return find_target(self.0, current_target);
		}

		None
	}
}

/// GRANDPA hard forks due to borked migration of session keys after a runtime
/// upgrade (at #1491596), the signalled authority set changes were invalid
/// (blank keys) and were impossible to finalize. The authorities for these
/// intermediary pending changes are replaced with a static list comprised of
/// w3f validators and randomly selected validators from the latest session (at
/// #1500988).
pub(crate) fn crust_hard_forks() -> Vec<(
	sp_finality_grandpa::SetId,
	(Hash, primitives::BlockNumber),
	sp_finality_grandpa::AuthorityList,
)> {
	use sp_core::crypto::Ss58Codec;
	use std::str::FromStr;

	let forks = vec![
		(
			602,
			"f5c6d5b13cf4890c4380913de9a06be7fd8b76498f24f9ba15173aa3ed4f5e74",
			2080256,
		),
		(
			602,
			"be11d588c11ae67ce8e7e44b348670936c3a89a58802a03ee1c4fd6ee2546e86",
			2080276,
		),
		(
			602,
			"d8b01f1e714b902d2ae6ac39b42f76635ed146aaa1d8856111ef8200e56b3f95",
			2080277,
		),
		(
			602,
			"880fe6451b2eb5f531e599f4b62b31a878a9af90cea8b2c1515668cf21da7d41",
			2080278,
		),
		(
			613,
			"880fe6451b2eb5f531e599f4b62b31a878a9af90cea8b2c1515668cf21da7d41",
			2080517,
		)
	];

	let authorities = vec![
		"cTGwC2Mi7zJV8U53cYD3D7qtm91nfdezCAaGkLfjbkNuGmDuH",
		"cTLFRe1m67s7CUohSURA8fZq6FrV5VibPfASHW79VYbdCZy1X",
		"cTK1dvN9qzVs5DpWhDkycY713LvrKeCozSbYvnsffEGGcsgi1",
		"cTMEj6J35SpoeScTy9A2hf9LzEVLxRWjKHGNTp91mxx86eiQU",
		"cTL737WsvHPLG5W3aYAPyiqSRH5ZhP2AWR42tFggnjHLixojh",
		"cTMek5ZJaXR2pYCorGScLe7YdmNtwGjoLA3mPTWe9WNV77fTS",
		"cTMikoHvwYKkctjbLf2YNba6seWWxx6XfEfLQy9y8vvSd7u61",
		"cTK7M613jY4gbTLDf8UMPRpZBBY9G3e5mitwQ9LutkuhGstLs",
		"cTLTjC3Nuciej9dDretA5CZ76xfwYekDPq1f9UFbtXQ9PVBrj",
		"cTHD2FBZu1xCfYnYaocGaggkkp8gN5qZkQPTvLAnPvycthTjR",
		"cTMBSYn5hGSmqNVuARhqfKDaUYcdguXALfvUGxbRTH8Nciqni",
		"cTK4JGqzrQ9HFVQi8y3ng4oHvGWbW6MMUS26E29RHeC4pP9m9",
		"cTLebYfzdBuJkXtgcCCAAWmTkvZVuF8CWfe7phJ6P5MgDWtHs",
		"cTKNypbYX6W3aePEFZXuKWwEtAjGkopzc7wfsSq3ngpZMzhFD",
		"cTMpXvPaTdEfYVsBuLEWAQ9AFkjbWXNigBpzzfey42a6zcQ8B",
		"cTMYPCP6Y2XuAw5bosd43aoQoKxunH8JzkBbf1Keu2GdCj1mS",
		"cTKjde7mX5ff4TkDXhjM47cPNRFWkogsnmLSQa79VvdpmGsxk",
		"cTHXayVsp6mEir2mj16EqPnB9bR6An2LYiq5dXGMmzFmqDChN",
		"cTM8i6T5i8oW9BWYMZiSPs8JdCcJ3JPvRP8GeFBukfSsVPiN5",
		"cTMsH8aHdWVDh5F3oMnujBP98ZST6xei7kHL4udW1B6EN89W8",
		"cTJM1DVy7H96Bbi8ubHqJBh3CGBo2KMoYu5RTEg1zdSou4URV",
		"cTJFrozX2BiZ4EroZFjo1gP9zMjvMfi4kHX12jWwZNE94Lcc4",
		"cTHJZ9mvc3uCYLDHkGFBL4ZqYsdHktzgCQ8Xvv4cgSsPsXi4x",
		"cTKpn8CaXrunfN3CxVw3mBodAxfpKaEuseqCoFrcvPQSyfuPN",
		"cTJXhn5EQowtnCgAxtfkcLg9vVS2qgpXPizAdZdowXpFM7GQH",
		"cTM3otRmd6zQLNRqEDQKAhTCmc9wNSxXAtBCb63vPP6zv82U3",
		"cTLecvNA9Agcb6eje9Wct14Fos5Ti4HTfwD96Q2APJtAto1bc",
		"cTMhG3HKnL7bjhnD7hD6PrDt5ERumjNitKQDHEwgWbpvSaTfz",
	];

	let authorities = authorities
		.into_iter()
		.map(|address| {
			(
				sp_finality_grandpa::AuthorityId::from_ss58check(address)
					.expect("hard fork authority addresses are static and they should be carefully defined; qed."),
				1,
			)
		})
		.collect::<Vec<_>>();

	forks
		.into_iter()
		.map(|(set_id, hash, number)| {
			let hash = Hash::from_str(hash)
				.expect("hard fork hashes are static and they should be carefully defined; qed.");

			(set_id, (hash, number), authorities.clone())
		})
		.collect()
}