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

use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::{fmt, iter};

use amplify::hex;
use amplify::hex::{FromHex, ToHex};
use commit_verify::{Digest, DigestExt, Sha256};
use derive::{
    Bip340Sig, ControlBlock, Derive, DeriveCompr, DeriveLegacy, DeriveScripts, DeriveSet,
    DeriveXOnly, DerivedScript, Idx, KeyOrigin, Keychain, LegacyPk, LegacySig, NormalIndex,
    RedeemScript, Sats, SigScript, TapDerivation, Terminal, Witness, WitnessScript, XOnlyPk,
    XpubAccount, XpubDerivable,
};
use indexmap::IndexMap;

use crate::{
    Pkh, Raw, Sh, ShMulti, ShScript, ShSortedMulti, ShWpkh, ShWsh, ShWshMulti, ShWshScript,
    ShWshSortedMulti, Tr, TrKey, TrMulti, TrScript, TrSortedMulti, Wpkh, Wsh, WshMulti, WshScript,
    WshSortedMulti,
};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(lowercase)]
pub enum SpkClass {
    Bare,
    P2pkh,
    P2sh,
    P2wpkh,
    P2wsh,
    P2tr,
}

impl SpkClass {
    pub const fn dust_limit(self) -> Sats {
        match self {
            SpkClass::Bare => Sats(0),
            SpkClass::P2pkh => Sats(546),
            SpkClass::P2sh => Sats(540),
            SpkClass::P2wpkh => Sats(294),
            SpkClass::P2wsh | SpkClass::P2tr => Sats(330),
        }
    }

    pub const fn is_taproot(self) -> bool {
        match self {
            SpkClass::Bare
            | SpkClass::P2pkh
            | SpkClass::P2sh
            | SpkClass::P2wpkh
            | SpkClass::P2wsh => false,
            SpkClass::P2tr => true,
        }
    }

    pub const fn is_segwit(self) -> bool {
        match self {
            SpkClass::Bare | SpkClass::P2pkh | SpkClass::P2sh => false,
            SpkClass::P2wpkh | SpkClass::P2wsh => true,
            SpkClass::P2tr => true,
        }
    }

    pub const fn is_segwit_v0(self) -> bool {
        match self {
            SpkClass::Bare | SpkClass::P2pkh | SpkClass::P2sh => false,
            SpkClass::P2wpkh | SpkClass::P2wsh => true,
            SpkClass::P2tr => false,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct LegacyKeySig {
    pub key: LegacyPk,
    pub sig: LegacySig,
}

impl LegacyKeySig {
    pub fn new(key: LegacyPk, sig: LegacySig) -> Self { LegacyKeySig { key, sig } }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct TaprootKeySig {
    pub key: XOnlyPk,
    pub sig: Bip340Sig,
}

impl TaprootKeySig {
    pub fn new(key: XOnlyPk, sig: Bip340Sig) -> Self { TaprootKeySig { key, sig } }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct DescrId(pub u64);

impl From<[u8; 8]> for DescrId {
    fn from(value: [u8; 8]) -> Self { Self(u64::from_le_bytes(value)) }
}

impl From<DescrId> for [u8; 8] {
    fn from(id: DescrId) -> Self { id.0.to_le_bytes() }
}

impl Display for DescrId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let val = self.0.to_le_bytes();
        f.write_str(&val[..4].to_hex())?;
        f.write_str("-")?;
        f.write_str(&val[4..].to_hex())
    }
}

impl FromStr for DescrId {
    type Err = hex::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 17 {
            return Err(hex::Error::InvalidLength(17, 16));
        }
        if s.as_bytes()[8] != b'-' {
            return Err(hex::Error::InvalidChar(s.as_bytes()[8]));
        }
        let s = s.replace("-", "");
        let val = <[u8; 8]>::from_hex(&s)?;
        Ok(Self::from(val))
    }
}

#[cfg(feature = "serde")]
mod _serde {
    pub use super::*;

    impl serde::Serialize for DescrId {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: serde::Serializer {
            if serializer.is_human_readable() {
                serializer.serialize_str(&self.to_string())
            } else {
                self.0.serialize(serializer)
            }
        }
    }

    impl<'de> serde::Deserialize<'de> for DescrId {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: serde::Deserializer<'de> {
            use serde::de::Error;
            if deserializer.is_human_readable() {
                let s = String::deserialize(deserializer)?;
                s.parse().map_err(D::Error::custom)
            } else {
                u64::deserialize(deserializer).map(Self)
            }
        }
    }
}

pub trait Descriptor<K = XpubDerivable, V = ()>: DeriveScripts + Clone + Display {
    fn id(&self) -> DescrId {
        let spk = self
            .derive(Keychain::OUTER, NormalIndex::ZERO)
            .next()
            .expect("at least one derivation must be available")
            .to_script_pubkey();
        let mut engine = Sha256::new_with_prefix(*b"wallet-descriptor");
        engine.input_with_len::<{ u64::MAX as usize }>(spk.as_slice());
        let digest = engine.finish();
        let mut id = [0u8; 8];
        id.copy_from_slice(&digest[..8]);
        DescrId(u64::from_le_bytes(id))
    }

    fn class(&self) -> SpkClass;
    #[inline]
    fn is_taproot(&self) -> bool { self.class().is_taproot() }
    #[inline]
    fn is_segwit(&self) -> bool { self.class().is_segwit() }

    fn keys<'a>(&'a self) -> impl Iterator<Item = &'a K>
    where K: 'a;
    fn vars<'a>(&'a self) -> impl Iterator<Item = &'a V>
    where V: 'a;
    fn xpubs(&self) -> impl Iterator<Item = &XpubAccount>;

    fn legacy_keyset(&self, terminal: Terminal) -> IndexMap<LegacyPk, KeyOrigin>;
    fn xonly_keyset(&self, terminal: Terminal) -> IndexMap<XOnlyPk, TapDerivation>;

    fn legacy_witness(
        &self,
        keysigs: IndexMap<&KeyOrigin, LegacyKeySig>,
        redeem_script: Option<RedeemScript>,
        witness_script: Option<WitnessScript>,
    ) -> Option<(SigScript, Option<Witness>)>;

    fn taproot_witness(
        &self,
        cb: Option<&ControlBlock>,
        keysigs: IndexMap<&KeyOrigin, TaprootKeySig>,
    ) -> Option<Witness>;
}

#[derive(Clone, Eq, PartialEq, Hash, Debug, From)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(
        rename_all = "camelCase",
        bound(
            serialize = "S::Legacy: serde::Serialize, S::Compr: serde::Serialize, S::XOnly: \
                         serde::Serialize",
            deserialize = "S::Legacy: serde::Deserialize<'de>, S::Compr: serde::Deserialize<'de>, \
                           S::XOnly: serde::Deserialize<'de>"
        )
    )
)]
#[non_exhaustive]
pub enum StdDescr<S: DeriveSet = XpubDerivable> {
    #[from]
    Raw(Raw<S::Legacy>),

    #[from]
    Pkh(Pkh<S::Legacy>),

    #[from]
    ShScript(ShScript<S::Legacy>),

    #[from]
    ShMulti(ShMulti<S::Legacy>),

    #[from]
    ShSortedMulti(ShSortedMulti<S::Legacy>),

    #[from]
    Wpkh(Wpkh<S::Compr>),

    #[from]
    WshScript(WshScript<S::Compr>),

    #[from]
    WshMulti(WshMulti<S::Compr>),

    #[from]
    WshSortedMulti(WshSortedMulti<S::Compr>),

    #[from]
    ShWpkh(ShWpkh<S::Compr>),

    #[from]
    ShWshScript(ShWshScript<S::Compr>),

    #[from]
    ShWshMulti(ShWshMulti<S::Compr>),

    #[from]
    ShWshSortedMulti(ShWshSortedMulti<S::Compr>),

    #[from]
    TrKey(TrKey<S::XOnly>),

    #[from]
    TrMulti(TrMulti<S::XOnly>),

    #[from]
    TrSortedMulti(TrSortedMulti<S::XOnly>),

    #[from]
    TrTree(TrScript<S::XOnly>),
}

impl<S: DeriveSet> From<Sh<S>> for StdDescr<S> {
    fn from(d: Sh<S>) -> Self {
        match d {
            Sh::Wpkh(d) => Self::ShWpkh(d),
            Sh::ShScript(d) => Self::ShScript(d),
            Sh::ShMulti(d) => Self::ShMulti(d),
            Sh::ShSortedMulti(d) => Self::ShSortedMulti(d),
            Sh::WshScript(d) => Self::ShWshScript(d),
            Sh::WshMulti(d) => Self::ShWshMulti(d),
            Sh::WshSortedMulti(d) => Self::ShWshSortedMulti(d),
        }
    }
}

impl<S: DeriveSet> From<Wsh<S::Compr>> for StdDescr<S> {
    fn from(d: Wsh<S::Compr>) -> Self {
        match d {
            Wsh::Script(d) => Self::WshScript(d),
            Wsh::Multi(d) => Self::WshMulti(d),
            Wsh::SortedMulti(d) => Self::WshSortedMulti(d),
        }
    }
}

impl<S: DeriveSet> From<ShWsh<S::Compr>> for StdDescr<S> {
    fn from(d: ShWsh<S::Compr>) -> Self {
        match d {
            ShWsh::Script(d) => Self::ShWshScript(d),
            ShWsh::Multi(d) => Self::ShWshMulti(d),
            ShWsh::SortedMulti(d) => Self::ShWshSortedMulti(d),
        }
    }
}

impl<S: DeriveSet> From<Tr<S::XOnly>> for StdDescr<S> {
    fn from(d: Tr<S::XOnly>) -> Self {
        match d {
            Tr::KeyOnly(d) => Self::TrKey(d),
            Tr::Script(d) => Self::TrTree(d),
            Tr::Multi(d) => Self::TrMulti(d),
            Tr::SortedMulti(d) => Self::TrSortedMulti(d),
        }
    }
}

impl<S: DeriveSet> Derive<DerivedScript> for StdDescr<S> {
    fn default_keychain(&self) -> Keychain {
        match self {
            StdDescr::Raw(d) => d.default_keychain(),
            StdDescr::Pkh(d) => d.default_keychain(),
            StdDescr::ShScript(d) => d.default_keychain(),
            StdDescr::ShMulti(d) => d.default_keychain(),
            StdDescr::ShSortedMulti(d) => d.default_keychain(),
            StdDescr::ShWpkh(d) => d.default_keychain(),
            StdDescr::Wpkh(d) => d.default_keychain(),
            StdDescr::WshScript(d) => d.default_keychain(),
            StdDescr::WshMulti(d) => d.default_keychain(),
            StdDescr::WshSortedMulti(d) => d.default_keychain(),
            StdDescr::ShWshScript(d) => d.default_keychain(),
            StdDescr::ShWshMulti(d) => d.default_keychain(),
            StdDescr::ShWshSortedMulti(d) => d.default_keychain(),
            StdDescr::TrKey(d) => d.default_keychain(),
            StdDescr::TrMulti(d) => d.default_keychain(),
            StdDescr::TrSortedMulti(d) => d.default_keychain(),
            StdDescr::TrTree(d) => d.default_keychain(),
        }
    }

    fn keychains(&self) -> BTreeSet<Keychain> {
        match self {
            StdDescr::Raw(d) => d.keychains(),
            StdDescr::Pkh(d) => d.keychains(),
            StdDescr::ShScript(d) => d.keychains(),
            StdDescr::ShMulti(d) => d.keychains(),
            StdDescr::ShSortedMulti(d) => d.keychains(),
            StdDescr::ShWpkh(d) => d.keychains(),
            StdDescr::Wpkh(d) => d.keychains(),
            StdDescr::WshScript(d) => d.keychains(),
            StdDescr::WshMulti(d) => d.keychains(),
            StdDescr::WshSortedMulti(d) => d.keychains(),
            StdDescr::ShWshScript(d) => d.keychains(),
            StdDescr::ShWshMulti(d) => d.keychains(),
            StdDescr::ShWshSortedMulti(d) => d.keychains(),
            StdDescr::TrKey(d) => d.keychains(),
            StdDescr::TrMulti(d) => d.keychains(),
            StdDescr::TrSortedMulti(d) => d.keychains(),
            StdDescr::TrTree(d) => d.keychains(),
        }
    }

    fn derive(
        &self,
        keychain: impl Into<Keychain>,
        index: impl Into<NormalIndex>,
    ) -> impl Iterator<Item = DerivedScript> {
        match self {
            StdDescr::Raw(d) => d.derive(keychain, index).collect::<Vec<_>>().into_iter(),
            StdDescr::Pkh(d) => d.derive(keychain, index).collect::<Vec<_>>().into_iter(),
            StdDescr::ShScript(d) => d.derive(keychain, index).collect::<Vec<_>>().into_iter(),
            StdDescr::ShMulti(d) => d.derive(keychain, index).collect::<Vec<_>>().into_iter(),
            StdDescr::ShSortedMulti(d) => d.derive(keychain, index).collect::<Vec<_>>().into_iter(),
            StdDescr::ShWpkh(d) => d.derive(keychain, index).collect::<Vec<_>>().into_iter(),
            StdDescr::Wpkh(d) => d.derive(keychain, index).collect::<Vec<_>>().into_iter(),
            StdDescr::WshScript(d) => d.derive(keychain, index).collect::<Vec<_>>().into_iter(),
            StdDescr::WshMulti(d) => d.derive(keychain, index).collect::<Vec<_>>().into_iter(),
            StdDescr::WshSortedMulti(d) => {
                d.derive(keychain, index).collect::<Vec<_>>().into_iter()
            }
            StdDescr::ShWshScript(d) => d.derive(keychain, index).collect::<Vec<_>>().into_iter(),
            StdDescr::ShWshMulti(d) => d.derive(keychain, index).collect::<Vec<_>>().into_iter(),
            StdDescr::ShWshSortedMulti(d) => {
                d.derive(keychain, index).collect::<Vec<_>>().into_iter()
            }
            StdDescr::TrKey(d) => d.derive(keychain, index).collect::<Vec<_>>().into_iter(),
            StdDescr::TrMulti(d) => d.derive(keychain, index).collect::<Vec<_>>().into_iter(),
            StdDescr::TrSortedMulti(d) => d.derive(keychain, index).collect::<Vec<_>>().into_iter(),
            StdDescr::TrTree(d) => d.derive(keychain, index).collect::<Vec<_>>().into_iter(),
        }
    }
}

impl<K: DeriveSet<Legacy = K, Compr = K, XOnly = K> + DeriveLegacy + DeriveCompr + DeriveXOnly>
    Descriptor<K> for StdDescr<K>
where Self: Derive<DerivedScript>
{
    fn class(&self) -> SpkClass {
        match self {
            StdDescr::Raw(d) => d.class(),
            StdDescr::Pkh(d) => d.class(),
            StdDescr::ShScript(d) => d.class(),
            StdDescr::ShMulti(d) => d.class(),
            StdDescr::ShSortedMulti(d) => d.class(),
            StdDescr::ShWpkh(d) => d.class(),
            StdDescr::Wpkh(d) => d.class(),
            StdDescr::WshScript(d) => d.class(),
            StdDescr::WshMulti(d) => d.class(),
            StdDescr::WshSortedMulti(d) => d.class(),
            StdDescr::ShWshScript(d) => d.class(),
            StdDescr::ShWshMulti(d) => d.class(),
            StdDescr::ShWshSortedMulti(d) => d.class(),
            StdDescr::TrKey(d) => d.class(),
            StdDescr::TrMulti(d) => d.class(),
            StdDescr::TrSortedMulti(d) => d.class(),
            StdDescr::TrTree(d) => d.class(),
        }
    }

    fn keys<'a>(&'a self) -> impl Iterator<Item = &'a K>
    where K: 'a {
        match self {
            StdDescr::Raw(d) => d.keys().collect::<Vec<_>>(),
            StdDescr::Pkh(d) => d.keys().collect::<Vec<_>>(),
            StdDescr::ShScript(d) => d.keys().collect::<Vec<_>>(),
            StdDescr::ShMulti(d) => d.keys().collect::<Vec<_>>(),
            StdDescr::ShSortedMulti(d) => d.keys().collect::<Vec<_>>(),
            StdDescr::ShWpkh(d) => d.keys().collect::<Vec<_>>(),
            StdDescr::Wpkh(d) => d.keys().collect::<Vec<_>>(),
            StdDescr::WshScript(d) => d.keys().collect::<Vec<_>>(),
            StdDescr::WshMulti(d) => d.keys().collect::<Vec<_>>(),
            StdDescr::WshSortedMulti(d) => d.keys().collect::<Vec<_>>(),
            StdDescr::ShWshScript(d) => d.keys().collect::<Vec<_>>(),
            StdDescr::ShWshMulti(d) => d.keys().collect::<Vec<_>>(),
            StdDescr::ShWshSortedMulti(d) => d.keys().collect::<Vec<_>>(),
            StdDescr::TrKey(d) => d.keys().collect::<Vec<_>>(),
            StdDescr::TrMulti(d) => d.keys().collect::<Vec<_>>(),
            StdDescr::TrSortedMulti(d) => d.keys().collect::<Vec<_>>(),
            StdDescr::TrTree(d) => d.keys().collect::<Vec<_>>(),
        }
        .into_iter()
    }

    fn vars<'a>(&'a self) -> impl Iterator<Item = &'a ()>
    where (): 'a {
        iter::empty()
    }

    fn xpubs(&self) -> impl Iterator<Item = &XpubAccount> {
        match self {
            StdDescr::Raw(d) => d.xpubs().collect::<Vec<_>>(),
            StdDescr::Pkh(d) => d.xpubs().collect::<Vec<_>>(),
            StdDescr::ShScript(d) => d.xpubs().collect::<Vec<_>>(),
            StdDescr::ShMulti(d) => d.xpubs().collect::<Vec<_>>(),
            StdDescr::ShSortedMulti(d) => d.xpubs().collect::<Vec<_>>(),
            StdDescr::ShWpkh(d) => d.xpubs().collect::<Vec<_>>(),
            StdDescr::Wpkh(d) => d.xpubs().collect::<Vec<_>>(),
            StdDescr::WshScript(d) => d.xpubs().collect::<Vec<_>>(),
            StdDescr::WshMulti(d) => d.xpubs().collect::<Vec<_>>(),
            StdDescr::WshSortedMulti(d) => d.xpubs().collect::<Vec<_>>(),
            StdDescr::ShWshScript(d) => d.xpubs().collect::<Vec<_>>(),
            StdDescr::ShWshMulti(d) => d.xpubs().collect::<Vec<_>>(),
            StdDescr::ShWshSortedMulti(d) => d.xpubs().collect::<Vec<_>>(),
            StdDescr::TrKey(d) => d.xpubs().collect::<Vec<_>>(),
            StdDescr::TrMulti(d) => d.xpubs().collect::<Vec<_>>(),
            StdDescr::TrSortedMulti(d) => d.xpubs().collect::<Vec<_>>(),
            StdDescr::TrTree(d) => d.xpubs().collect::<Vec<_>>(),
        }
        .into_iter()
    }

    fn legacy_keyset(&self, terminal: Terminal) -> IndexMap<LegacyPk, KeyOrigin> {
        match self {
            StdDescr::Raw(d) => d.legacy_keyset(terminal),
            StdDescr::Pkh(d) => d.legacy_keyset(terminal),
            StdDescr::ShScript(d) => d.legacy_keyset(terminal),
            StdDescr::ShMulti(d) => d.legacy_keyset(terminal),
            StdDescr::ShSortedMulti(d) => d.legacy_keyset(terminal),
            StdDescr::ShWpkh(d) => d.legacy_keyset(terminal),
            StdDescr::Wpkh(d) => d.legacy_keyset(terminal),
            StdDescr::WshScript(d) => d.legacy_keyset(terminal),
            StdDescr::WshMulti(d) => d.legacy_keyset(terminal),
            StdDescr::WshSortedMulti(d) => d.legacy_keyset(terminal),
            StdDescr::ShWshScript(d) => d.legacy_keyset(terminal),
            StdDescr::ShWshMulti(d) => d.legacy_keyset(terminal),
            StdDescr::ShWshSortedMulti(d) => d.legacy_keyset(terminal),
            StdDescr::TrKey(d) => d.legacy_keyset(terminal),
            StdDescr::TrMulti(d) => d.legacy_keyset(terminal),
            StdDescr::TrSortedMulti(d) => d.legacy_keyset(terminal),
            StdDescr::TrTree(d) => d.legacy_keyset(terminal),
        }
    }

    fn xonly_keyset(&self, terminal: Terminal) -> IndexMap<XOnlyPk, TapDerivation> {
        match self {
            StdDescr::Raw(d) => d.xonly_keyset(terminal),
            StdDescr::Pkh(d) => d.xonly_keyset(terminal),
            StdDescr::ShScript(d) => d.xonly_keyset(terminal),
            StdDescr::ShMulti(d) => d.xonly_keyset(terminal),
            StdDescr::ShSortedMulti(d) => d.xonly_keyset(terminal),
            StdDescr::ShWpkh(d) => d.xonly_keyset(terminal),
            StdDescr::Wpkh(d) => d.xonly_keyset(terminal),
            StdDescr::WshScript(d) => d.xonly_keyset(terminal),
            StdDescr::WshMulti(d) => d.xonly_keyset(terminal),
            StdDescr::WshSortedMulti(d) => d.xonly_keyset(terminal),
            StdDescr::ShWshScript(d) => d.xonly_keyset(terminal),
            StdDescr::ShWshMulti(d) => d.xonly_keyset(terminal),
            StdDescr::ShWshSortedMulti(d) => d.xonly_keyset(terminal),
            StdDescr::TrKey(d) => d.xonly_keyset(terminal),
            StdDescr::TrMulti(d) => d.xonly_keyset(terminal),
            StdDescr::TrSortedMulti(d) => d.xonly_keyset(terminal),
            StdDescr::TrTree(d) => d.xonly_keyset(terminal),
        }
    }

    fn legacy_witness(
        &self,
        keysigs: IndexMap<&KeyOrigin, LegacyKeySig>,
        redeem_script: Option<RedeemScript>,
        witness_script: Option<WitnessScript>,
    ) -> Option<(SigScript, Option<Witness>)> {
        match self {
            StdDescr::Raw(d) => d.legacy_witness(keysigs, redeem_script, witness_script),
            StdDescr::Pkh(d) => d.legacy_witness(keysigs, redeem_script, witness_script),
            StdDescr::ShScript(d) => d.legacy_witness(keysigs, redeem_script, witness_script),
            StdDescr::ShMulti(d) => d.legacy_witness(keysigs, redeem_script, witness_script),
            StdDescr::ShSortedMulti(d) => d.legacy_witness(keysigs, redeem_script, witness_script),
            StdDescr::ShWpkh(d) => d.legacy_witness(keysigs, redeem_script, witness_script),
            StdDescr::Wpkh(d) => d.legacy_witness(keysigs, redeem_script, witness_script),
            StdDescr::WshScript(d) => d.legacy_witness(keysigs, redeem_script, witness_script),
            StdDescr::WshMulti(d) => d.legacy_witness(keysigs, redeem_script, witness_script),
            StdDescr::WshSortedMulti(d) => d.legacy_witness(keysigs, redeem_script, witness_script),
            StdDescr::ShWshScript(d) => d.legacy_witness(keysigs, redeem_script, witness_script),
            StdDescr::ShWshMulti(d) => d.legacy_witness(keysigs, redeem_script, witness_script),
            StdDescr::ShWshSortedMulti(d) => {
                d.legacy_witness(keysigs, redeem_script, witness_script)
            }
            StdDescr::TrKey(d) => d.legacy_witness(keysigs, redeem_script, witness_script),
            StdDescr::TrMulti(d) => d.legacy_witness(keysigs, redeem_script, witness_script),
            StdDescr::TrSortedMulti(d) => d.legacy_witness(keysigs, redeem_script, witness_script),
            StdDescr::TrTree(d) => d.legacy_witness(keysigs, redeem_script, witness_script),
        }
    }

    fn taproot_witness(
        &self,
        cb: Option<&ControlBlock>,
        keysigs: IndexMap<&KeyOrigin, TaprootKeySig>,
    ) -> Option<Witness> {
        match self {
            StdDescr::Raw(d) => d.taproot_witness(cb, keysigs),
            StdDescr::Pkh(d) => d.taproot_witness(cb, keysigs),
            StdDescr::ShScript(d) => d.taproot_witness(cb, keysigs),
            StdDescr::ShMulti(d) => d.taproot_witness(cb, keysigs),
            StdDescr::ShSortedMulti(d) => d.taproot_witness(cb, keysigs),
            StdDescr::ShWpkh(d) => d.taproot_witness(cb, keysigs),
            StdDescr::Wpkh(d) => d.taproot_witness(cb, keysigs),
            StdDescr::WshScript(d) => d.taproot_witness(cb, keysigs),
            StdDescr::WshMulti(d) => d.taproot_witness(cb, keysigs),
            StdDescr::WshSortedMulti(d) => d.taproot_witness(cb, keysigs),
            StdDescr::ShWshScript(d) => d.taproot_witness(cb, keysigs),
            StdDescr::ShWshMulti(d) => d.taproot_witness(cb, keysigs),
            StdDescr::ShWshSortedMulti(d) => d.taproot_witness(cb, keysigs),
            StdDescr::TrKey(d) => d.taproot_witness(cb, keysigs),
            StdDescr::TrMulti(d) => d.taproot_witness(cb, keysigs),
            StdDescr::TrSortedMulti(d) => d.taproot_witness(cb, keysigs),
            StdDescr::TrTree(d) => d.taproot_witness(cb, keysigs),
        }
    }
}

impl<S: DeriveSet> Display for StdDescr<S>
where
    S::Legacy: Display,
    S::Compr: Display,
    S::XOnly: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            StdDescr::Raw(d) => Display::fmt(d, f),
            StdDescr::Pkh(d) => Display::fmt(d, f),
            StdDescr::ShScript(d) => Display::fmt(d, f),
            StdDescr::ShMulti(d) => Display::fmt(d, f),
            StdDescr::ShSortedMulti(d) => Display::fmt(d, f),
            StdDescr::ShWpkh(d) => Display::fmt(d, f),
            StdDescr::Wpkh(d) => Display::fmt(d, f),
            StdDescr::WshScript(d) => Display::fmt(d, f),
            StdDescr::WshMulti(d) => Display::fmt(d, f),
            StdDescr::WshSortedMulti(d) => Display::fmt(d, f),
            StdDescr::ShWshScript(d) => Display::fmt(d, f),
            StdDescr::ShWshMulti(d) => Display::fmt(d, f),
            StdDescr::ShWshSortedMulti(d) => Display::fmt(d, f),
            StdDescr::TrKey(d) => Display::fmt(d, f),
            StdDescr::TrMulti(d) => Display::fmt(d, f),
            StdDescr::TrSortedMulti(d) => Display::fmt(d, f),
            StdDescr::TrTree(d) => Display::fmt(d, f),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn descr_id_baid64() {
        let descr_id = DescrId::from([0xde, 0xad, 0xbe, 0xef, 0xbe, 0xad, 0xca, 0xfe]);
        let s = descr_id.to_string();
        assert_eq!(s, "deadbeef-beadcafe");
        assert_eq!(DescrId::from_str(&s).unwrap(), descr_id);
    }
}
