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
#[macro_use]
extern crate indexmap;
#[cfg(feature = "strict_encoding")]
#[macro_use]
extern crate strict_encoding;
#[cfg(feature = "serde")]
#[macro_use]
extern crate serde;

mod data;
mod keys;
mod maps;
mod coders;
#[cfg(feature = "client-side-validation")]
mod csval;
pub mod constructor;
mod sign;

pub use coders::{Decode, DecodeError, Encode, PsbtError};
pub use constructor::{
    Beneficiary, BeneficiaryParseError, ChangeInfo, ConstructionError, Payment, PsbtConstructor,
    PsbtMeta, TxParams, Utxo,
};
#[cfg(feature = "client-side-validation")]
pub use csval::*;
pub use data::{
    Input, ModifiableFlags, Output, Prevout, Psbt, PsbtParseError, UnfinalizedInputs, Unmodifiable,
    UnsignedTx, UnsignedTxIn,
};
pub use keys::{GlobalKey, InputKey, KeyPair, KeyType, OutputKey, PropKey};
pub use maps::{KeyAlreadyPresent, KeyData, KeyMap, Map, MapName, ValueData};
pub use sign::{Rejected, SignError, Signer};

#[cfg(feature = "strict_encoding")]
pub const LIB_NAME_PSBT: &str = "Psbt";

#[derive(Copy, Clone, Eq, PartialEq, Debug, Display, Error)]
#[display("unsupported version of PSBT v{0}")]
pub struct PsbtUnsupportedVer(u32);

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display)]
#[cfg_attr(feature = "serde", derive(Serialize), serde(rename_all = "camelCase"))]
pub enum PsbtVer {
    #[display("v0")]
    V0 = 0,
    #[display("v2")]
    V2 = 2,
}

impl PsbtVer {
    pub const fn try_from_standard_u32(v: u32) -> Result<Self, PsbtUnsupportedVer> {
        Ok(match v {
            0 => Self::V0,
            2 => Self::V2,
            wrong => return Err(PsbtUnsupportedVer(wrong)),
        })
    }

    pub const fn to_standard_u32(&self) -> u32 { *self as u32 }

    pub const fn max() -> Self {
        // this is a special syntax construct to get compiler error each time we add a new version
        // and not to forget upgrade the result of this method
        match Self::V0 {
            PsbtVer::V0 | PsbtVer::V2 => PsbtVer::V2,
        }
    }
}

impl TryFrom<usize> for PsbtVer {
    type Error = PsbtUnsupportedVer;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::V0),
            2 => Ok(Self::V2),
            _ => Err(PsbtUnsupportedVer(value as u32)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn psbt_try_from_numbers() {
        assert_eq!(PsbtVer::try_from(0), Ok(PsbtVer::V0));
        assert_eq!(PsbtVer::try_from(2), Ok(PsbtVer::V2));
        assert_eq!(PsbtVer::try_from(1), Err(PsbtUnsupportedVer(1)));
    }
}
