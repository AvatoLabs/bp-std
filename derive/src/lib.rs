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

#[macro_use]
extern crate amplify;
#[cfg(feature = "serde")]
#[macro_use]
extern crate serde;

mod index;
mod path;
mod xkey;
mod derive;
pub mod taptree;
mod sign;

pub use bc::*;
pub use derive::{
    Derive, DeriveCompr, DeriveKey, DeriveLegacy, DeriveScripts, DeriveSet, DeriveXOnly,
    DerivedAddr, DerivedAddrParseError, DerivedScript, Keychain, Terminal, TerminalParseError,
};
pub use index::{
    DerivationIndex, HardenedIndex, Idx, IdxBase, IndexError, IndexParseError, NormalIndex,
    HARDENED_INDEX_BOUNDARY,
};
pub use invoice::*;
pub use path::{DerivationParseError, DerivationPath, DerivationSeg, SegParseError};
pub use sign::Sign;
pub use taptree::{
    ControlBlockFactory, FinalizedTree, InvalidTree, LeafInfo, TapDerivation, TapTree,
    TapTreeBuilder, UnfinalizedTree,
};
pub use xkey::{
    ChainCode, KeyOrigin, OriginParseError, XkeyAccountError, XkeyDecodeError, XkeyMeta,
    XkeyOrigin, XkeyParseError, Xpriv, XprivAccount, XprivCore, Xpub, XpubAccount, XpubCore,
    XpubDerivable, XpubFp, XpubId, XPRIV_MAINNET_MAGIC, XPRIV_TESTNET_MAGIC,
};
