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

use core::fmt::Display;
use core::str::FromStr;
use std::collections::BTreeSet;
use std::fmt::Debug;
use std::iter;

use amplify::confinement::ConfinedVec;
use amplify::num::u4;
use derive::{
    Derive, DeriveCompr, DeriveKey, DeriveLegacy, DeriveSet, DeriveXOnly, Keychain, NormalIndex,
    XkeyDecodeError, XpubAccount,
};

use crate::compiler::{DescrAst, DescrParseError, ScriptExpr};
use crate::{
    Pkh, Sh, ShMulti, ShScript, ShSortedMulti, ShWpkh, ShWsh, ShWshMulti, ShWshScript,
    ShWshSortedMulti, StdDescr, Tr, TrKey, TrMulti, TrScript, TrSortedMulti, Wpkh, Wsh, WshMulti,
    WshScript, WshSortedMulti,
};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum DescrExpr {
    Script,
    Lit,
    Key,
    Tree,
    VariadicKey,
    VariadicLit,
}

impl DescrExpr {
    pub fn check_expr<K: Display + FromStr>(&self, expr: &DescrAst<K>) -> bool
    where K::Err: core::error::Error {
        matches!(
            (self, expr),
            (DescrExpr::Lit | DescrExpr::VariadicLit, DescrAst::Lit(_, _))
                | (DescrExpr::Key | DescrExpr::VariadicKey, DescrAst::Key(_, _))
                | (DescrExpr::Script, DescrAst::Script(_))
                | (DescrExpr::Tree, DescrAst::Tree(_))
        )
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Display)]
#[display("")]
pub struct NoKey;
impl DeriveSet for NoKey {
    type Legacy = Self;
    type Compr = Self;
    type XOnly = Self;
}
impl<K> Derive<K> for NoKey {
    fn default_keychain(&self) -> Keychain {
        unreachable!();
    }
    fn keychains(&self) -> BTreeSet<Keychain> {
        unreachable!();
    }
    fn derive(
        &self,
        _keychain: impl Into<Keychain>,
        _index: impl Into<NormalIndex>,
    ) -> impl Iterator<Item = K> {
        iter::empty()
    }
}
impl<K> DeriveKey<K> for NoKey {
    fn xpub_spec(&self) -> &XpubAccount {
        unreachable!();
    }
}
impl FromStr for NoKey {
    type Err = XkeyDecodeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Err(XkeyDecodeError::WrongExtendedKeyLength(s.len()))
    }
}

pub fn check_forms<'s, K: Display + FromStr>(
    ast: ScriptExpr<'s, K>,
    ident: &str,
    form: &[DescrExpr],
) -> Option<Vec<DescrAst<'s, K>>>
where
    K::Err: core::error::Error,
{
    if ast.name != ident {
        return None;
    }
    let mut iter1 = form.iter();
    let mut iter2 = ast.children.iter();
    if iter1.by_ref().zip(iter2.by_ref()).any(|(a, b)| !a.check_expr(b)) {
        return None;
    }
    if iter1.count() > 0 {
        return None;
    }
    if form.last() == Some(&DescrExpr::VariadicKey) {
        if iter2.any(|d| !DescrExpr::Key.check_expr(d)) {
            return None;
        }
    } else if form.last() == Some(&DescrExpr::VariadicLit) {
        if iter2.any(|d| !DescrExpr::Lit.check_expr(d)) {
            return None;
        }
    } else if iter2.count() > 0 {
        return None;
    }
    Some(ast.children)
}

////////////////////////////////////////
// Key-only pre-taproot

impl<K: DeriveLegacy + FromStr> FromStr for Pkh<K>
where K::Err: core::error::Error
{
    type Err = DescrParseError<K::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ast = ScriptExpr::<K>::from_str(s)?;
        let mut form = check_forms(ast, "pkh", &[DescrExpr::Key][..])
            .ok_or(DescrParseError::InvalidArgs("pkh"))?;
        let Some(DescrAst::Key(key, _)) = form.pop() else {
            unreachable!();
        };
        Ok(Pkh::from(key))
    }
}

impl<K: DeriveCompr + FromStr> FromStr for Wpkh<K>
where K::Err: core::error::Error
{
    type Err = DescrParseError<K::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ast = ScriptExpr::<K>::from_str(s)?;
        let mut form = check_forms(ast, "wpkh", &[DescrExpr::Key][..])
            .ok_or(DescrParseError::InvalidArgs("wpkh"))?;
        let Some(DescrAst::Key(key, _)) = form.pop() else {
            unreachable!();
        };
        Ok(Wpkh::from(key))
    }
}

impl<K: DeriveCompr + FromStr> FromStr for ShWpkh<K>
where K::Err: core::error::Error
{
    type Err = DescrParseError<K::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ast = ScriptExpr::<K>::from_str(s)?;
        let mut form = check_forms(ast, "sh", &[DescrExpr::Script][..])
            .ok_or(DescrParseError::InvalidArgs("sh"))?;
        let Some(DescrAst::Script(inner)) = form.pop() else {
            unreachable!();
        };

        let mut form = check_forms(*inner, "wpkh", &[DescrExpr::Key][..])
            .ok_or(DescrParseError::InvalidArgs("wpkh"))?;
        let Some(DescrAst::Key(key, _)) = form.pop() else {
            unreachable!();
        };
        Ok(ShWpkh::from(key))
    }
}

////////////////////////////////////////
// Multisigs pre-taproot

fn parse_multi_form<K: Display + FromStr>(
    s: &str,
    outer: &'static str,
    medium: Option<&'static str>,
    inner: &'static str,
) -> Result<(u4, ConfinedVec<K, 1, 16>), DescrParseError<K::Err>>
where
    K::Err: core::error::Error,
{
    let ast = ScriptExpr::<K>::from_str(s)?;

    let mut form = check_forms(ast, outer, &[DescrExpr::Script][..])
        .ok_or(DescrParseError::InvalidArgs(outer))?;
    let Some(DescrAst::Script(mut script)) = form.pop() else {
        unreachable!();
    };

    if let Some(medium) = medium {
        let mut form = check_forms(*script, medium, &[DescrExpr::Script][..])
            .ok_or(DescrParseError::InvalidArgs(medium))?;
        let Some(DescrAst::Script(script2)) = form.pop() else {
            unreachable!();
        };
        script = script2;
    }

    let mut form = check_forms(*script, inner, &[DescrExpr::Lit, DescrExpr::VariadicKey][..])
        .ok_or(DescrParseError::InvalidArgs(inner))?;
    let DescrAst::Lit(thresh, _) = form.remove(0) else {
        unreachable!();
    };
    let threshold = u4::from_str(thresh)?;
    let keys = ConfinedVec::try_from_iter(form.into_iter().map(|el| {
        let DescrAst::Key(key, _) = el else {
            unreachable!()
        };
        key
    }))?;
    Ok((threshold, keys))
}

impl<K: DeriveLegacy + FromStr> FromStr for ShMulti<K>
where K::Err: core::error::Error
{
    type Err = DescrParseError<K::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (threshold, keys) = parse_multi_form(s, "sh", None, "multi")?;
        Ok(ShMulti { threshold, keys })
    }
}

impl<K: DeriveLegacy + FromStr> FromStr for ShSortedMulti<K>
where K::Err: core::error::Error
{
    type Err = DescrParseError<K::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (threshold, keys) = parse_multi_form(s, "sh", None, "sortedmulti")?;
        Ok(ShSortedMulti { threshold, keys })
    }
}

impl<K: DeriveCompr + FromStr> FromStr for WshMulti<K>
where K::Err: core::error::Error
{
    type Err = DescrParseError<K::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (threshold, keys) = parse_multi_form(s, "wsh", None, "multi")?;
        Ok(WshMulti { threshold, keys })
    }
}

impl<K: DeriveCompr + FromStr> FromStr for WshSortedMulti<K>
where K::Err: core::error::Error
{
    type Err = DescrParseError<K::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (threshold, keys) = parse_multi_form(s, "wsh", None, "sortedmulti")?;
        Ok(WshSortedMulti { threshold, keys })
    }
}

impl<K: DeriveCompr + FromStr> FromStr for ShWshMulti<K>
where K::Err: core::error::Error
{
    type Err = DescrParseError<K::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (threshold, keys) = parse_multi_form(s, "sh", Some("wsh"), "multi")?;
        Ok(ShWshMulti { threshold, keys })
    }
}

impl<K: DeriveCompr + FromStr> FromStr for ShWshSortedMulti<K>
where K::Err: core::error::Error
{
    type Err = DescrParseError<K::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (threshold, keys) = parse_multi_form(s, "sh", Some("wsh"), "sortedmulti")?;
        Ok(ShWshSortedMulti { threshold, keys })
    }
}

////////////////////////////////////////
// Scripts pre-taproot

// TODO: Implement with support for script templates and miniscript

impl<K: DeriveLegacy + FromStr> FromStr for ShScript<K>
where K::Err: core::error::Error
{
    type Err = DescrParseError<K::Err>;

    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        Err(DescrParseError::NotSupported("scripts"))
    }
}

impl<K: DeriveCompr + FromStr> FromStr for WshScript<K>
where K::Err: core::error::Error
{
    type Err = DescrParseError<K::Err>;

    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        Err(DescrParseError::NotSupported("scripts"))
    }
}

impl<K: DeriveCompr + FromStr> FromStr for ShWshScript<K>
where K::Err: core::error::Error
{
    type Err = DescrParseError<K::Err>;

    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        Err(DescrParseError::NotSupported("scripts"))
    }
}

////////////////////////////////////////
// Taproot

impl<K: DeriveXOnly + FromStr> FromStr for TrKey<K>
where K::Err: core::error::Error
{
    type Err = DescrParseError<K::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ast = ScriptExpr::<K>::from_str(s)?;
        let mut form = check_forms(ast, "tr", &[DescrExpr::Key][..])
            .ok_or(DescrParseError::InvalidArgs("tr"))?;
        let Some(DescrAst::Key(key, _)) = form.pop() else {
            unreachable!();
        };
        Ok(TrKey::from(key))
    }
}

fn parse_tr_form<K: Display + FromStr>(
    s: &str,
    inner: &'static str,
) -> Result<(K, u16, ConfinedVec<K, 1, 999>), DescrParseError<K::Err>>
where
    K::Err: core::error::Error,
{
    let ast = ScriptExpr::<K>::from_str(s)?;

    let mut form = check_forms(ast, "tr", &[DescrExpr::Key, DescrExpr::Script][..])
        .ok_or(DescrParseError::InvalidArgs("tr"))?;
    let Some(DescrAst::Script(script)) = form.pop() else {
        unreachable!();
    };
    let Some(DescrAst::Key(internal_key, _)) = form.pop() else {
        unreachable!();
    };

    let mut form = check_forms(*script, inner, &[DescrExpr::Lit, DescrExpr::VariadicKey][..])
        .ok_or(DescrParseError::InvalidArgs(inner))?;
    let DescrAst::Lit(thresh, _) = form.remove(0) else {
        unreachable!();
    };
    let threshold = u16::from_str(thresh)?;
    let script_keys = ConfinedVec::try_from_iter(form.into_iter().map(|el| {
        let DescrAst::Key(key, _) = el else {
            unreachable!()
        };
        key
    }))?;
    Ok((internal_key, threshold, script_keys))
}

impl<K: DeriveXOnly + FromStr> FromStr for TrMulti<K>
where K::Err: core::error::Error
{
    type Err = DescrParseError<K::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (internal_key, threshold, script_keys) = parse_tr_form(s, "multi_a")?;
        Ok(TrMulti {
            internal_key,
            threshold,
            script_keys,
        })
    }
}

impl<K: DeriveXOnly + FromStr> FromStr for TrSortedMulti<K>
where K::Err: core::error::Error
{
    type Err = DescrParseError<K::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (internal_key, threshold, script_keys) = parse_tr_form(s, "sortedmulti_a")?;
        Ok(TrSortedMulti {
            internal_key,
            threshold,
            script_keys,
        })
    }
}

impl<K: DeriveXOnly + FromStr> FromStr for TrScript<K>
where K::Err: core::error::Error
{
    type Err = DescrParseError<K::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ast = ScriptExpr::<K>::from_str(s)?;

        let mut form = check_forms(ast, "tr", &[DescrExpr::Key, DescrExpr::Tree][..])
            .ok_or(DescrParseError::InvalidArgs("tr"))?;
        let Some(DescrAst::Key(_internal_key, _)) = form.pop() else {
            unreachable!();
        };
        let Some(DescrAst::Tree(_tree)) = form.pop() else {
            unreachable!();
        };

        // TODO: Process taproot tree

        Err(DescrParseError::NotSupported("scripts"))
    }
}

////////////////////////////////////////
// Combinators

fn trim_start_expr<'s>(s: &'s str, expr: &'static str) -> Option<&'s str> {
    s.trim_start().strip_prefix(expr).and_then(|rest| rest.trim_start().strip_prefix("("))
}

impl<K: DeriveSet + Display + FromStr> FromStr for Sh<K>
where
    K::Err: core::error::Error,
    K::Legacy: Display + FromStr<Err = K::Err>,
    K::Compr: Display + FromStr<Err = K::Err>,
{
    type Err = DescrParseError<K::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some(rest) = trim_start_expr(s, "sh") else {
            return Err(DescrParseError::NoRequiredScript("sh"));
        };
        if rest.starts_with("wsh") {
            Ok(ShWsh::from_str(s)?.into())
        } else if rest.starts_with("sortedmulti") {
            ShSortedMulti::from_str(s).map(Sh::ShSortedMulti)
        } else if rest.starts_with("multi") {
            ShMulti::from_str(s).map(Sh::ShMulti)
        } else {
            ShScript::from_str(s).map(Sh::ShScript)
        }
    }
}

impl<K: DeriveCompr + FromStr> FromStr for Wsh<K>
where K::Err: core::error::Error
{
    type Err = DescrParseError<K::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some(rest) = trim_start_expr(s, "wsh") else {
            return Err(DescrParseError::NoRequiredScript("wsh"));
        };
        let rest = rest.trim_start();
        if rest.starts_with("multi") {
            WshMulti::from_str(s).map(Wsh::Multi)
        } else if rest.starts_with("sortedmulti") {
            WshSortedMulti::from_str(s).map(Wsh::SortedMulti)
        } else {
            WshScript::from_str(s).map(Wsh::Script)
        }
    }
}

impl<K: DeriveCompr + FromStr> FromStr for ShWsh<K>
where K::Err: core::error::Error
{
    type Err = DescrParseError<K::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some(rest) = trim_start_expr(s, "sh") else {
            return Err(DescrParseError::NoRequiredScript("sh"));
        };
        let Some(rest) = trim_start_expr(rest, "wsh") else {
            return Err(DescrParseError::NoRequiredScript("wsh"));
        };
        if rest.starts_with("multi") {
            ShWshMulti::from_str(s).map(ShWsh::Multi)
        } else if rest.starts_with("sortedmulti") {
            ShWshSortedMulti::from_str(s).map(ShWsh::SortedMulti)
        } else {
            ShWshScript::from_str(s).map(ShWsh::Script)
        }
    }
}

impl<K: DeriveXOnly + FromStr> FromStr for Tr<K>
where K::Err: core::error::Error
{
    type Err = DescrParseError<K::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(tr_key) = TrKey::from_str(s) {
            Ok(Self::KeyOnly(tr_key))
        } else if let Ok(tr_multi) = TrMulti::from_str(s) {
            Ok(Self::Multi(tr_multi))
        } else if let Ok(tr_sorted_multi) = TrSortedMulti::from_str(s) {
            Ok(Self::SortedMulti(tr_sorted_multi))
        } else {
            TrScript::from_str(s).map(Self::Script)
        }
    }
}

impl<K: DeriveSet + Display + FromStr> FromStr for StdDescr<K>
where
    K::Err: core::error::Error,
    K::Legacy: Display + FromStr<Err = K::Err>,
    K::Compr: Display + FromStr<Err = K::Err>,
    K::XOnly: Display + FromStr<Err = K::Err>,
{
    type Err = DescrParseError<K::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim_start() {
            s if s.starts_with("pkh") => Self::Pkh(Pkh::from_str(s)?),
            s if s.starts_with("wpkh") => Self::Wpkh(Wpkh::from_str(s)?),

            s if s.starts_with("sh") => Sh::from_str(s)?.into(),
            s if s.starts_with("wsh") => Wsh::from_str(s)?.into(),
            s if s.starts_with("tr") => Tr::from_str(s)?.into(),

            _ => return Err(DescrParseError::InvalidScriptExpr(s.to_owned())),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::error::Error;
    use std::iter;

    use derive::{Derive, DeriveKey, Keychain, NormalIndex, XkeyDecodeError, XpubAccount};

    use super::*;
    use crate::Descriptor;

    #[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Display)]
    #[display("KEY")]
    struct DumbKey;
    impl DeriveSet for DumbKey {
        type Legacy = Self;
        type Compr = Self;
        type XOnly = Self;
    }
    impl<K> Derive<K> for DumbKey {
        fn default_keychain(&self) -> Keychain {
            unreachable!();
        }
        fn keychains(&self) -> BTreeSet<Keychain> {
            unreachable!();
        }
        fn derive(
            &self,
            _keychain: impl Into<Keychain>,
            _index: impl Into<NormalIndex>,
        ) -> impl Iterator<Item = K> {
            iter::empty()
        }
    }
    impl<K> DeriveKey<K> for DumbKey {
        fn xpub_spec(&self) -> &XpubAccount {
            unreachable!();
        }
    }
    impl FromStr for DumbKey {
        type Err = XkeyDecodeError;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if s != "KEY" {
                Err(XkeyDecodeError::InvalidSecretKey)
            } else {
                Ok(DumbKey)
            }
        }
    }

    fn roundtrip<D: Descriptor<DumbKey> + FromStr + Into<StdDescr<DumbKey>>>(s: &str, expect: D)
    where D::Err: Error {
        let expect = expect.into();
        let d1 = StdDescr::from_str(s).unwrap();
        let d2 = D::from_str(s).unwrap();
        assert_eq!(s, d1.to_string());
        assert_eq!(s, d2.to_string());
        assert_eq!(d1, expect);
        assert_eq!(d2.into(), expect);
    }

    #[test]
    fn pkh() { roundtrip("pkh(KEY)", Pkh::from(DumbKey)); }
    #[test]
    fn wpkh() { roundtrip("wpkh(KEY)", Wpkh::from(DumbKey)); }

    #[test]
    fn sh_multi() {
        roundtrip("sh(multi(2,KEY,KEY,KEY))", ShMulti::new_checked(2, [DumbKey, DumbKey, DumbKey]));
    }
    #[test]
    fn wsh_multi() {
        roundtrip(
            "wsh(multi(2,KEY,KEY,KEY))",
            WshMulti::new_checked(2, [DumbKey, DumbKey, DumbKey]),
        );
    }
    #[test]
    fn sh_wsh_multi() {
        roundtrip(
            "sh(wsh(multi(2,KEY,KEY,KEY)))",
            ShWshMulti::new_checked(2, [DumbKey, DumbKey, DumbKey]),
        );
    }

    #[test]
    fn sh_sortedmulti() {
        roundtrip(
            "sh(sortedmulti(2,KEY,KEY,KEY))",
            ShSortedMulti::new_checked(2, [DumbKey, DumbKey, DumbKey]),
        );
    }
    #[test]
    fn wsh_sortedmulti() {
        roundtrip(
            "wsh(sortedmulti(2,KEY,KEY,KEY))",
            WshSortedMulti::new_checked(2, [DumbKey, DumbKey, DumbKey]),
        );
    }
    #[test]
    fn sh_wsh_sortedmulti() {
        roundtrip(
            "sh(wsh(sortedmulti(2,KEY,KEY,KEY)))",
            ShWshSortedMulti::new_checked(2, [DumbKey, DumbKey, DumbKey]),
        );
    }

    #[test]
    fn tr_key_only() { roundtrip("tr(KEY)", TrKey::from(DumbKey)); }

    #[test]
    fn tr_multi_a() {
        roundtrip(
            "tr(KEY,multi_a(2,KEY,KEY,KEY))",
            TrMulti::new_checked(DumbKey, 2, [DumbKey, DumbKey, DumbKey]),
        );
    }
    #[test]
    fn tr_sortedmulti_a() {
        roundtrip(
            "tr(KEY,sortedmulti_a(2,KEY,KEY,KEY))",
            TrSortedMulti::new_checked(DumbKey, 2, [DumbKey, DumbKey, DumbKey]),
        );
    }
}
