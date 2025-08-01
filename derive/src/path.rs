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

use core::fmt::{self, Display, Formatter};
use core::num::ParseIntError;
use core::ops::Index;
use core::str::FromStr;
use std::collections::BTreeSet;

use amplify::confinement;
use amplify::confinement::{Confined, NonEmptyOrdSet};

use crate::{DerivationIndex, HardenedIndex, Idx, IdxBase, IndexParseError, NormalIndex, Terminal};

#[derive(Clone, Eq, PartialEq, Debug, Display, Error)]
#[display(doc_comments)]
pub enum DerivationParseError {
    /// unable to parse derivation path '{0}' - {1}
    InvalidIndex(String, IndexParseError),
    /// invalid derivation path format '{0}'
    InvalidFormat(String),
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct DerivationSeg<I: IdxBase = NormalIndex>(NonEmptyOrdSet<I, 8>);

impl<I: IdxBase> From<&'static [I]> for DerivationSeg<I> {
    fn from(indexes: &'static [I]) -> Self {
        Self(Confined::from_iter_checked(indexes.iter().copied()))
    }
}

impl<I: IdxBase> From<[I; 2]> for DerivationSeg<I> {
    fn from(indexes: [I; 2]) -> Self { Self(Confined::from_iter_checked(indexes)) }
}

impl<I: IdxBase> From<[I; 3]> for DerivationSeg<I> {
    fn from(indexes: [I; 3]) -> Self { Self(Confined::from_iter_checked(indexes)) }
}

impl<I: IdxBase> From<[I; 4]> for DerivationSeg<I> {
    fn from(indexes: [I; 4]) -> Self { Self(Confined::from_iter_checked(indexes)) }
}

impl<I: IdxBase> DerivationSeg<I> {
    pub fn new(index: I) -> Self { DerivationSeg(NonEmptyOrdSet::with(index)) }

    pub fn with(iter: impl IntoIterator<Item = I>) -> Result<Self, confinement::Error> {
        Confined::try_from_iter(iter).map(DerivationSeg)
    }

    #[inline]
    pub fn count(&self) -> u8 { self.0.len() as u8 }

    #[inline]
    pub fn is_distinct(&self, other: &Self) -> bool { self.0.is_disjoint(&other.0) }

    #[inline]
    pub fn at(&self, index: u8) -> Option<I> { self.0.iter().nth(index as usize).copied() }

    #[inline]
    pub fn first(&self) -> I {
        *self
            .0
            .first()
            .expect("confined type guarantees that there is at least one item in the collection")
    }

    #[inline]
    pub fn into_set(self) -> BTreeSet<I> { self.0.release() }

    #[inline]
    pub fn to_set(&self) -> BTreeSet<I> { self.0.to_unconfined() }

    #[inline]
    pub fn as_set(&self) -> &BTreeSet<I> { self.0.as_unconfined() }
}

impl DerivationSeg<NormalIndex> {
    pub fn standard() -> Self {
        DerivationSeg(NonEmptyOrdSet::from_iter_checked([NormalIndex::ZERO, NormalIndex::ONE]))
    }
}

impl<I: IdxBase> Index<u8> for DerivationSeg<I> {
    type Output = I;

    fn index(&self, index: u8) -> &Self::Output {
        self.0
            .iter()
            .nth(index as usize)
            .expect("requested position in derivation segment exceeds its length")
    }
}

impl<I: IdxBase + Display> Display for DerivationSeg<I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.count() == 1 {
            write!(f, "{}", self[0])
        } else {
            f.write_str("<")?;
            let mut first = true;
            for index in &self.0 {
                if !first {
                    f.write_str(";")?;
                }
                write!(f, "{index}")?;
                first = false;
            }
            f.write_str(">")
        }
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Display, Error, From)]
#[display(doc_comments)]
pub enum SegParseError {
    /// derivation contains invalid index - {0}.
    #[from]
    #[from(ParseIntError)]
    InvalidFormat(IndexParseError),

    /// derivation segment contains too many variants.
    #[from]
    Confinement(confinement::Error),
}

impl<I: IdxBase> FromStr for DerivationSeg<I>
where
    I: FromStr,
    SegParseError: From<I::Err>,
{
    type Err = SegParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let t = s.trim_start_matches('<').trim_end_matches('>');
        if t.len() + 2 == s.len() {
            let set = t.split(';').map(I::from_str).collect::<Result<BTreeSet<_>, _>>()?;
            Ok(Self(Confined::try_from_iter(set)?))
        } else {
            Ok(Self(I::from_str(s).map(Confined::with)?))
        }
    }
}

/// Derivation path that consisting only of single type of segments.
///
/// Useful in specifying concrete derivation from a provided extended public key
/// without extended private key accessible.
///
/// Type guarantees that the number of derivation path segments is non-zero.
#[derive(Wrapper, WrapperMut, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Default, Debug, From)]
#[wrapper(Deref)]
#[wrapper_mut(DerefMut)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
pub struct DerivationPath<I = DerivationIndex>(Vec<I>);

impl<I: Clone, const LEN: usize> From<[I; LEN]> for DerivationPath<I> {
    fn from(path: [I; LEN]) -> Self { Self(path.to_vec()) }
}

impl<I: Clone> From<&[I]> for DerivationPath<I> {
    fn from(path: &[I]) -> Self { Self(path.to_vec()) }
}

impl<I> AsRef<[I]> for DerivationPath<I> {
    fn as_ref(&self) -> &[I] { self.0.as_ref() }
}

impl<I: Display> Display for DerivationPath<I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for segment in &self.0 {
            f.write_str("/")?;
            Display::fmt(segment, f)?;
        }
        Ok(())
    }
}

impl<I: FromStr> FromStr for DerivationPath<I>
where IndexParseError: From<<I as FromStr>::Err>
{
    type Err = DerivationParseError;

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('/') {
            s = &s[1..];
        }
        let inner = s
            .split('/')
            .map(I::from_str)
            .collect::<Result<Vec<_>, I::Err>>()
            .map_err(|err| DerivationParseError::InvalidIndex(s.to_owned(), err.into()))?;
        if inner.is_empty() {
            return Err(DerivationParseError::InvalidFormat(s.to_owned()));
        }
        Ok(Self(inner))
    }
}

impl<I> IntoIterator for DerivationPath<I> {
    type Item = I;
    type IntoIter = std::vec::IntoIter<I>;

    fn into_iter(self) -> Self::IntoIter { self.0.into_iter() }
}

impl<'path, I: Copy> IntoIterator for &'path DerivationPath<I> {
    type Item = I;
    type IntoIter = std::iter::Copied<std::slice::Iter<'path, I>>;

    fn into_iter(self) -> Self::IntoIter { self.0.iter().copied() }
}

impl<I> FromIterator<I> for DerivationPath<I> {
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self { Self(iter.into_iter().collect()) }
}

impl<I: Idx> DerivationPath<I> {
    /// Constructs empty derivation path.
    pub fn new() -> Self { Self(vec![]) }

    pub fn with_capacity(capacity: usize) -> Self { Self(Vec::with_capacity(capacity)) }

    pub fn terminal(&self) -> Option<Terminal> {
        let mut iter = self.iter().rev();
        let index = iter.next()?;
        if index.is_hardened() {
            return None;
        }
        let index = NormalIndex::normal(index.child_number() as u16);
        let keychain = iter.next()?;
        if keychain.is_hardened() {
            return None;
        }
        let keychain = u8::try_from(keychain.child_number()).ok()?;
        Some(Terminal::new(keychain, index))
    }

    pub fn starts_with<I2>(&self, master: impl AsRef<[I2]>) -> bool
    where
        I: Into<DerivationIndex>,
        I2: Idx + Into<DerivationIndex>,
    {
        let master = master.as_ref();
        self.shared_prefix(master) == master.len()
    }

    pub fn hardened_prefix(&self) -> DerivationPath<HardenedIndex> {
        self.0
            .iter()
            .take_while(|i| i.is_hardened())
            .map(|i| HardenedIndex::try_from_index(i.index()).expect("hardened index"))
            .collect()
    }

    pub fn shared_prefix<I2>(&self, master: impl AsRef<[I2]>) -> usize
    where
        I: Into<DerivationIndex>,
        I2: Idx + Into<DerivationIndex>,
    {
        let master = master.as_ref();
        if master.len() <= self.len() {
            let shared = self
                .iter()
                .zip(master)
                .take_while(|(i1, i2)| (**i1).into() == (**i2).into())
                .count();
            if shared == master.len() {
                return shared;
            }
        }
        0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::HardenedIndex;

    #[test]
    fn altstr() {
        let path1 = DerivationPath::<HardenedIndex>::from_str("86h/1h/0h").unwrap();
        let path2 = DerivationPath::<HardenedIndex>::from_str("86'/1'/0'").unwrap();
        let path3 = DerivationPath::<HardenedIndex>::from_str("86'/1h/0h").unwrap();
        assert_eq!(path1, path2);
        assert_eq!(path1, path3);
    }
}
