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

use std::str::FromStr;

use bc::BlockHash;

use crate::AddressNetwork;

/// Bitcoin network used by the address
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase",)
)]
#[display(lowercase)]
pub enum Network {
    /// Bitcoin mainnet
    #[display("bitcoin")]
    Mainnet,

    /// Bitcoin testnet3
    Testnet3,

    /// Bitcoin testnet4
    Testnet4,

    /// Bitcoin signet
    Signet,

    /// Bitcoin regtest networks
    Regtest,
}

impl Network {
    /// Detects whether the network is a kind of test network (testnet, signet,
    /// regtest).
    pub fn is_testnet(self) -> bool { self != Self::Mainnet }

    pub const fn genesis_hash(self) -> BlockHash {
        match self {
            Network::Mainnet => BlockHash::GENESIS_MAINNET,
            Network::Testnet3 => BlockHash::GENESIS_TESTNET3,
            Network::Testnet4 => BlockHash::GENESIS_TESTNET4,
            Network::Signet => BlockHash::GENESIS_SIGNET,
            Network::Regtest => BlockHash::GENESIS_REGTEST,
        }
    }
}

impl From<Network> for AddressNetwork {
    fn from(network: Network) -> Self {
        match network {
            Network::Mainnet => AddressNetwork::Mainnet,
            Network::Testnet3 | Network::Testnet4 | Network::Signet => AddressNetwork::Testnet,
            Network::Regtest => AddressNetwork::Regtest,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Display, Error)]
#[display("unknown genesis block hash '{0}'")]
pub struct UnknownGenesisBlock(pub BlockHash);

impl TryFrom<BlockHash> for Network {
    type Error = UnknownGenesisBlock;

    fn try_from(hash: BlockHash) -> Result<Self, Self::Error> {
        match hash {
            BlockHash::GENESIS_MAINNET => Ok(Network::Mainnet),
            BlockHash::GENESIS_TESTNET3 => Ok(Network::Testnet3),
            BlockHash::GENESIS_TESTNET4 => Ok(Network::Testnet4),
            BlockHash::GENESIS_SIGNET => Ok(Network::Signet),
            BlockHash::GENESIS_REGTEST => Ok(Network::Regtest),
            _ => Err(UnknownGenesisBlock(hash)),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Display, Error)]
#[display("unknown bitcoin network '{0}'")]
pub struct UnknownNetwork(pub String);

impl FromStr for Network {
    type Err = UnknownNetwork;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "bitcoin" | "mainnet" => Network::Mainnet,
            "testnet" | "testnet3" => Network::Testnet3,
            "testnet4" => Network::Testnet4,
            "signet" => Network::Signet,
            "regtest" => Network::Regtest,
            other => return Err(UnknownNetwork(other.to_owned())),
        })
    }
}
