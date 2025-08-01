// Modern, minimalistic & standard-compliant Bitcoin library.
//
// SPDX-License-Identifier: Apache-2.0
//
// Designed in 2019-2025 by Dr Maxim Orlovsky <orlovsky@lnp-bp.org>
// Written in 2024-2025 by Dr Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// Copyright (C) 2019-2024 LNP/BP Standards Association, Switzerland.
// Copyright (C) 2024-2025 LNP/BP Labs, Institute for Distributed and Cognitive Systems (InDCS).
// Copyright (C) 2019-2025 Dr Maxim Orlovsky.
// All rights under the above copyrights are reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fmt::{self, Display, Formatter, Write};
use std::ops::Deref;
use std::{slice, vec};

use amplify::num::u7;
use bc::{
    ControlBlock, InternalPk, LeafScript, OutputPk, Parity, TapLeafHash, TapMerklePath,
    TapNodeHash, TapScript,
};
use commit_verify::merkle::MerkleBuoy;

use crate::{KeyOrigin, Terminal, XkeyOrigin};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Display, Error, From)]
pub enum InvalidTree {
    #[from]
    #[display(doc_comments)]
    Unfinalized(UnfinalizedTree),

    #[from(FinalizedTree)]
    #[display("tap tree contains too many script leaves which doesn't fit a single Merkle tree")]
    MountainRange,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Display, Error)]
#[display("can't add more leaves to an already finalized tap tree")]
pub struct FinalizedTree;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Display, Error)]
#[display(
    "unfinalized tap tree containing leaves at level {0} which can't commit into a single Merkle \
     root"
)]
pub struct UnfinalizedTree(pub u7);

#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct TapTreeBuilder<L = LeafScript> {
    leaves: Vec<LeafInfo<L>>,
    buoy: MerkleBuoy<u7>,
    finalized: bool,
}

impl<L> TapTreeBuilder<L> {
    pub fn new() -> Self {
        Self {
            leaves: none!(),
            buoy: default!(),
            finalized: false,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            leaves: Vec::with_capacity(capacity),
            buoy: zero!(),
            finalized: false,
        }
    }

    pub fn is_finalized(&self) -> bool { self.finalized }

    pub fn with_leaf(mut self, leaf: LeafInfo<L>) -> Result<Self, FinalizedTree> {
        self.push_leaf(leaf)?;
        Ok(self)
    }

    pub fn push_leaf(&mut self, leaf: LeafInfo<L>) -> Result<bool, FinalizedTree> {
        if self.finalized {
            return Err(FinalizedTree);
        }
        let depth = leaf.depth;
        self.leaves.push(leaf);
        self.buoy.push(depth);
        if self.buoy.level() == u7::ZERO {
            self.finalized = true
        }
        Ok(self.finalized)
    }

    pub fn finish(self) -> Result<TapTree<L>, UnfinalizedTree> {
        if !self.finalized {
            return Err(UnfinalizedTree(self.buoy.level()));
        }
        Ok(TapTree(self.leaves))
    }
}

/// Non-empty taproot script tree.
#[derive(Clone, Eq, PartialEq, Hash, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(transparent))]
pub struct TapTree<L = LeafScript>(Vec<LeafInfo<L>>);

impl<L> Deref for TapTree<L> {
    type Target = Vec<LeafInfo<L>>;
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl<L> IntoIterator for TapTree<L> {
    type Item = LeafInfo<L>;
    type IntoIter = vec::IntoIter<LeafInfo<L>>;

    fn into_iter(self) -> Self::IntoIter { self.0.into_iter() }
}

impl<'a, L> IntoIterator for &'a TapTree<L> {
    type Item = &'a LeafInfo<L>;
    type IntoIter = slice::Iter<'a, LeafInfo<L>>;

    fn into_iter(self) -> Self::IntoIter { self.0.iter() }
}

impl TapTree {
    pub fn with_single_leaf(leaf: impl Into<LeafScript>) -> TapTree {
        Self(vec![LeafInfo {
            depth: u7::ZERO,
            script: leaf.into(),
        }])
    }

    pub fn merkle_root(&self) -> TapNodeHash {
        if self.0.len() == 1 {
            TapLeafHash::with_leaf_script(&self.0[0].script).into()
        } else {
            todo!("#10 implement TapTree::merkle_root for trees with more than one leaf")
        }
    }
}

impl<L> TapTree<L> {
    pub fn from_leaves(leaves: impl IntoIterator<Item = LeafInfo<L>>) -> Result<Self, InvalidTree> {
        let mut builder = TapTreeBuilder::<L>::new();
        for leaf in leaves {
            builder.push_leaf(leaf)?;
        }
        builder.finish().map_err(InvalidTree::from)
    }

    pub fn from_builder(builder: TapTreeBuilder<L>) -> Result<Self, UnfinalizedTree> {
        builder.finish()
    }

    pub fn into_vec(self) -> Vec<LeafInfo<L>> { self.0 }

    pub fn map<M>(self, f: impl Fn(L) -> M) -> TapTree<M> {
        TapTree(
            self.into_iter()
                .map(|leaf| LeafInfo {
                    depth: leaf.depth,
                    script: f(leaf.script),
                })
                .collect(),
        )
    }
}

impl<L: Display> Display for TapTree<L> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut buoy = MerkleBuoy::<u7>::default();

        let mut depth = u7::ZERO;
        for leaf in &self.0 {
            for _ in depth.into_u8()..leaf.depth.into_u8() {
                f.write_char('{')?;
            }
            buoy.push(leaf.depth);
            if depth == leaf.depth {
                f.write_char(',')?;
            }
            depth = leaf.depth;
            for _ in buoy.level().into_u8()..depth.into_u8() {
                f.write_char('}')?;
            }
            debug_assert_ne!(buoy.level(), u7::ZERO);
        }
        debug_assert_eq!(buoy.level(), u7::ZERO);
        Ok(())
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
pub struct LeafInfo<L = LeafScript> {
    pub depth: u7,
    pub script: L,
}

impl LeafInfo<LeafScript> {
    pub fn tap_script(depth: u7, script: TapScript) -> Self {
        LeafInfo {
            depth,
            script: LeafScript::from_tap_script(script),
        }
    }
}

#[derive(Getters, Clone, Eq, PartialEq, Debug)]
#[getter(as_copy)]
pub struct ControlBlockFactory {
    internal_pk: InternalPk,
    output_pk: OutputPk,
    parity: Parity,
    merkle_root: TapNodeHash,

    #[getter(skip)]
    merkle_path: TapMerklePath,
    #[getter(skip)]
    remaining_leaves: Vec<LeafInfo>,
}

impl ControlBlockFactory {
    #[inline]
    pub fn with(internal_pk: InternalPk, tap_tree: TapTree) -> Self {
        let merkle_root = tap_tree.merkle_root();
        let (output_pk, parity) = internal_pk.to_output_pk(Some(merkle_root));
        ControlBlockFactory {
            internal_pk,
            output_pk,
            parity,
            merkle_root,
            merkle_path: empty!(),
            remaining_leaves: tap_tree.into_vec(),
        }
    }

    #[inline]
    pub fn into_remaining_leaves(self) -> Vec<LeafInfo> { self.remaining_leaves }
}

impl Iterator for ControlBlockFactory {
    type Item = (ControlBlock, LeafScript);

    fn next(&mut self) -> Option<Self::Item> {
        let leaf = self.remaining_leaves.pop()?;
        let leaf_script = leaf.script;
        let control_block = ControlBlock::with(
            leaf_script.version,
            self.internal_pk,
            self.parity,
            self.merkle_path.clone(),
        );
        Some((control_block, leaf_script))
    }
}

/// A compact size unsigned integer representing the number of leaf hashes, followed by a list
/// of leaf hashes, followed by the 4 byte master key fingerprint concatenated with the
/// derivation path of the public key. The derivation path is represented as 32-bit little
/// endian unsigned integer indexes concatenated with each other. Public keys are those needed
/// to spend this output. The leaf hashes are of the leaves which involve this public key. The
/// internal key does not have leaf hashes, so can be indicated with a hashes len of 0.
/// Finalizers should remove this field after `PSBT_IN_FINAL_SCRIPTWITNESS` is constructed.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
pub struct TapDerivation {
    pub leaf_hashes: Vec<TapLeafHash>,
    pub origin: KeyOrigin,
}

impl TapDerivation {
    pub fn with_internal_pk(xpub_origin: XkeyOrigin, terminal: Terminal) -> Self {
        let origin = KeyOrigin::with(xpub_origin, terminal);
        TapDerivation {
            leaf_hashes: empty!(),
            origin,
        }
    }
}
