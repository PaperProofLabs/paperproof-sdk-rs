// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Deployment {
    pub name: String,
    pub network: String,
    pub rpc_url: String,
    pub protocol_version: String,
    pub packages: DeploymentPackages,
    pub objects: DeploymentObjects,
    pub coin_types: CoinTypes,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct DeploymentPackages {
    pub pprf: String,
    pub governance_original: String,
    pub governance: String,
    pub comments: String,
    pub publishing: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct DeploymentObjects {
    pub root: String,
    pub type_registry: String,
    pub fee_manager: String,
    pub governance_vault: String,
    pub governance_config: String,
    pub clock: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct CoinTypes {
    pub pprf: String,
    pub wal: String,
    pub sui: String,
}

pub fn mainnet_deployment() -> Deployment {
    Deployment {
        name: "paperproof-mainnet-2026-05-08".to_string(),
        network: "mainnet".to_string(),
        rpc_url: "https://fullnode.mainnet.sui.io:443".to_string(),
        protocol_version: "publishing-v2-governance-v2-comments-v2".to_string(),
        packages: DeploymentPackages {
            pprf: "0x5d2ec9829a9e116de7c2008281a90b96690beb2252af120ad05a25fe13fae0da".to_string(),
            governance_original:
                "0x75923624e354789e995537e88afaab698bd405a61f91926e3f8837fb7cc6b5cf".to_string(),
            governance: "0xc1ced3b8ae5281eeeb8cdb5527978e294c54f14a7fd8d65e7e9502d4ffffb87e"
                .to_string(),
            comments: "0xaef346fc40bf20af62f4bbbc1608ba2272e80e4ba3d716634026baa589e9aeba"
                .to_string(),
            publishing: "0xe67a6956f37c3182354189d9b77ca14058694aad82522da0c6cb91cfddee4782"
                .to_string(),
        },
        objects: DeploymentObjects {
            root: "0x7dc6c78b276825499a2204b060394e80b81196eb1f77d2036b503a2cca15dd78".to_string(),
            type_registry: "0x966ffa24d0a96b34267b62c628f39c830afc9de25438b6502835fa8a3815d6b5"
                .to_string(),
            fee_manager: "0x7bb8360ea1fa50f923628c929b8726b00eb8968c6a678acde71f97ae146e9249"
                .to_string(),
            governance_vault: "0x0df35aa53ef37f8ca8f6a6280d743effa6e0bfc613c5c6c0a78318ad4a38f875"
                .to_string(),
            governance_config: "0x7ed018db6b2cd7c32692a1c33543fb90d9c36add1226f93cbeb2a8fb10955dfa"
                .to_string(),
            clock: "0x6".to_string(),
        },
        coin_types: CoinTypes {
            pprf: "0x5d2ec9829a9e116de7c2008281a90b96690beb2252af120ad05a25fe13fae0da::pprf::PPRF"
                .to_string(),
            wal: "0x356a26eb9e012a68958082340d4c4116e7f55615cf27affcff209cf0ae544f59::wal::WAL"
                .to_string(),
            sui: "0x2::sui::SUI".to_string(),
        },
    }
}

pub static MAINNET_DEPLOYMENT: std::sync::LazyLock<Deployment> =
    std::sync::LazyLock::new(mainnet_deployment);
