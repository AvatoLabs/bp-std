// Modern, minimalistic & standard-compliant cold wallet library.
//
// SPDX-License-Identifier: Apache-2.0
//
// Written in 2020-2023 by
//     Dr Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// Copyright (C) 2020-2023 LNP/BP Standards Association. All rights reserved.
// Copyright (C) 2020-2023 Dr Maxim Orlovsky. All rights reserved.
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

use std::iter;

use bp::secp256k1;
use bp::secp256k1::ecdsa;

/// This type is consensus valid but an input including it would prevent the transaction from
/// being relayed on today's Bitcoin network.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Display, Error)]
#[display("non-standard SIGHASH_TYPE value {0:#X}")]
pub struct NonStandardSighashType(pub u32);

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate", rename_all = "camelCase")
)]
#[repr(u8)]
pub enum SighashFlag {
    /// 0x1: Sign all outputs.
    All = 0x01,
    /// 0x2: Sign no outputs --- anyone can choose the destination.
    None = 0x02,
    /// 0x3: Sign the output whose index matches this input's index. If none exists,
    /// sign the hash `0000000000000000000000000000000000000000000000000000000000000001`.
    /// (This rule is probably an unintentional C++ism, but it's consensus so we have
    /// to follow it.)
    Single = 0x03,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate", rename_all = "camelCase")
)]
pub struct SighashType {
    pub flag: SighashFlag,
    pub anyone_can_pay: bool,
}

impl SighashType {
    pub const fn all() -> Self {
        SighashType {
            flag: SighashFlag::All,
            anyone_can_pay: false,
        }
    }
    pub const fn none() -> Self {
        SighashType {
            flag: SighashFlag::None,
            anyone_can_pay: false,
        }
    }
    pub const fn single() -> Self {
        SighashType {
            flag: SighashFlag::Single,
            anyone_can_pay: false,
        }
    }

    pub const fn all_anyone_can_pay() -> Self {
        SighashType {
            flag: SighashFlag::All,
            anyone_can_pay: true,
        }
    }
    pub const fn none_anyone_can_pay() -> Self {
        SighashType {
            flag: SighashFlag::None,
            anyone_can_pay: true,
        }
    }
    pub const fn single_anyone_can_pay() -> Self {
        SighashType {
            flag: SighashFlag::Single,
            anyone_can_pay: true,
        }
    }

    /// Creates a [`SighashType`] from a raw `u32`.
    ///
    /// **Note**: this replicates consensus behaviour, for current standardness rules correctness
    /// you probably want [`Self::from_standard`].
    ///
    /// This might cause unexpected behavior because it does not roundtrip. That is,
    /// `LegacySighashType::from_consensus(n) as u32 != n` for non-standard values of `n`. While
    /// verifying signatures, the user should retain the `n` and use it compute the signature hash
    /// message.
    pub fn from_consensus(n: u32) -> SighashType {
        // In Bitcoin Core, the SignatureHash function will mask the (int32) value with
        // 0x1f to (apparently) deactivate ACP when checking for SINGLE and NONE bits.
        // We however want to be matching also against on ACP-masked ALL, SINGLE, and NONE.
        // So here we re-activate ACP.
        let mask = 0x1f | 0x80;
        let (flag, anyone_can_pay) = match n & mask {
            // "real" sighashes
            0x01 => (SighashFlag::All, false),
            0x02 => (SighashFlag::None, false),
            0x03 => (SighashFlag::Single, false),
            0x81 => (SighashFlag::All, true),
            0x82 => (SighashFlag::None, true),
            0x83 => (SighashFlag::Single, true),
            // catchalls
            x if x & 0x80 == 0x80 => (SighashFlag::All, true),
            _ => (SighashFlag::All, false),
        };
        SighashType {
            flag,
            anyone_can_pay,
        }
    }

    /// Creates a [`SighashType`] from a raw `u32`.
    ///
    /// # Errors
    ///
    /// If `n` is a non-standard sighash value.
    pub fn from_standard(n: u32) -> Result<SighashType, NonStandardSighashType> {
        let (flag, anyone_can_pay) = match n {
            // Standard sighashes, see https://github.com/bitcoin/bitcoin/blob/b805dbb0b9c90dadef0424e5b3bf86ac308e103e/src/script/interpreter.cpp#L189-L198
            0x01 => (SighashFlag::All, false),
            0x02 => (SighashFlag::None, false),
            0x03 => (SighashFlag::Single, false),
            0x81 => (SighashFlag::All, true),
            0x82 => (SighashFlag::None, true),
            0x83 => (SighashFlag::Single, true),
            non_standard => return Err(NonStandardSighashType(non_standard)),
        };
        Ok(SighashType {
            flag,
            anyone_can_pay,
        })
    }

    pub fn from_psbt_u8(val: u8) -> Result<SighashType, NonStandardSighashType> {
        Self::from_standard(val as u32)
    }

    /// Converts [`SighashType`] to a `u32` sighash flag.
    ///
    /// The returned value is guaranteed to be a valid according to standardness rules.
    #[inline]
    pub const fn into_u32(self) -> u32 { self.into_u8() as u32 }

    pub const fn into_u8(self) -> u8 {
        let flag = self.flag as u8;
        let mask = (self.anyone_can_pay as u8) << 7;
        flag | mask
    }
}

/// An ECDSA signature-related error.
#[derive(Clone, PartialEq, Eq, Debug, Display, Error, From)]
#[display(doc_comments)]
pub enum EcdsaSigError {
    /// Non-standard sighash type.
    #[display(inner)]
    #[from]
    SighashType(NonStandardSighashType),

    /// empty signature.
    EmptySignature,

    /// invalid signature DER encoding.
    #[from(secp256k1::Error)]
    DerEncoding,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate", rename_all = "camelCase")
)]
pub struct EcdsaSig {
    /// The underlying ECDSA Signature
    pub sig: ecdsa::Signature,
    /// The corresponding hash type
    pub sighash_type: SighashType,
}

impl EcdsaSig {
    /// Constructs an ECDSA bitcoin signature for [`SighashType::All`].
    pub fn sighash_all(sig: ecdsa::Signature) -> EcdsaSig {
        EcdsaSig {
            sig,
            sighash_type: SighashType::all(),
        }
    }

    /// Deserializes from slice following the standardness rules for [`SighashType`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, EcdsaSigError> {
        let (hash_ty, sig) = bytes.split_last().ok_or(EcdsaSigError::EmptySignature)?;
        let sighash_type = SighashType::from_standard(*hash_ty as u32)?;
        let sig = ecdsa::Signature::from_der(sig)?;
        Ok(EcdsaSig { sig, sighash_type })
    }

    /// Serializes an ECDSA signature (inner secp256k1 signature in DER format) into `Vec`.
    pub fn to_vec(self) -> Vec<u8> {
        // TODO: add support to serialize to a writer to SerializedSig
        self.sig
            .serialize_der()
            .iter()
            .copied()
            .chain(iter::once(self.sighash_type.into_u8()))
            .collect()
    }
}
