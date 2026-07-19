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
    #[serde(default)]
    pub package_history: DeploymentPackageHistory,
    pub objects: DeploymentObjects,
    pub coin_types: CoinTypes,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct DeploymentPackages {
    pub pprf: String,
    pub governance_original: String,
    pub governance: String,
    pub comments: String,
    pub publishing_original: Option<String>,
    pub publishing: String,
    pub controller: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct DeploymentPackageHistory {
    #[serde(default)]
    pub pprf: Vec<String>,
    #[serde(default)]
    pub governance: Vec<String>,
    #[serde(default)]
    pub comments: Vec<String>,
    #[serde(default)]
    pub publishing: Vec<String>,
    #[serde(default)]
    pub controller: Vec<String>,
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
        name: "paperproof-mainnet-2026-05-13".to_string(),
        network: "mainnet".to_string(),
        rpc_url: "https://fullnode.mainnet.sui.io:443".to_string(),
        protocol_version: "publishing-v4-comments-v3-controller-v1-governance-v2".to_string(),
        packages: DeploymentPackages {
            pprf: "0x5d2ec9829a9e116de7c2008281a90b96690beb2252af120ad05a25fe13fae0da".to_string(),
            governance_original:
                "0x75923624e354789e995537e88afaab698bd405a61f91926e3f8837fb7cc6b5cf".to_string(),
            governance: "0xc1ced3b8ae5281eeeb8cdb5527978e294c54f14a7fd8d65e7e9502d4ffffb87e"
                .to_string(),
            comments: "0x4962dda7d3033a6dd23724721ee38ca16720e8949b94d39826d24eb09f39e0a6"
                .to_string(),
            publishing_original: Some(
                "0xe67a6956f37c3182354189d9b77ca14058694aad82522da0c6cb91cfddee4782"
                    .to_string(),
            ),
            publishing: "0xfd9ea70eef5220dbba93ae2bf7cd077d4ddebe03d585ebc7ad536ed3ba500660"
                .to_string(),
            controller: Some(
                "0xe68fef47337eb2ee970431fae9519c4b2bb9f4505a3d14b6b91fdfc6aae3b75c"
                    .to_string(),
            ),
        },
        package_history: DeploymentPackageHistory {
            pprf: vec![
                "0x5d2ec9829a9e116de7c2008281a90b96690beb2252af120ad05a25fe13fae0da"
                    .to_string(),
            ],
            governance: vec![
                "0x75923624e354789e995537e88afaab698bd405a61f91926e3f8837fb7cc6b5cf"
                    .to_string(),
                "0xc1ced3b8ae5281eeeb8cdb5527978e294c54f14a7fd8d65e7e9502d4ffffb87e"
                    .to_string(),
            ],
            comments: vec![
                "0xaef346fc40bf20af62f4bbbc1608ba2272e80e4ba3d716634026baa589e9aeba"
                    .to_string(),
                "0x4962dda7d3033a6dd23724721ee38ca16720e8949b94d39826d24eb09f39e0a6"
                    .to_string(),
            ],
            publishing: vec![
                "0xe67a6956f37c3182354189d9b77ca14058694aad82522da0c6cb91cfddee4782"
                    .to_string(),
                "0xc9a75e4514db2a37df6f95b4e2b329c065ac6089953bd2c1c0a0c389835bd3d8"
                    .to_string(),
                "0xfd9ea70eef5220dbba93ae2bf7cd077d4ddebe03d585ebc7ad536ed3ba500660"
                    .to_string(),
            ],
            controller: vec![
                "0xe68fef47337eb2ee970431fae9519c4b2bb9f4505a3d14b6b91fdfc6aae3b75c"
                    .to_string(),
            ],
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeploymentPackageFamily {
    Pprf,
    Governance,
    Comments,
    Publishing,
    Controller,
}

pub fn deployment_package_ids(
    deployment: &Deployment,
    family: DeploymentPackageFamily,
) -> Vec<String> {
    fn push_unique(out: &mut Vec<String>, value: Option<&str>) {
        let Some(value) = value else {
            return;
        };
        if !out.iter().any(|existing| existing.eq_ignore_ascii_case(value)) {
            out.push(value.to_string());
        }
    }

    let mut packages = Vec::new();
    match family {
        DeploymentPackageFamily::Pprf => {
            for id in &deployment.package_history.pprf {
                push_unique(&mut packages, Some(id));
            }
            push_unique(&mut packages, Some(&deployment.packages.pprf));
        }
        DeploymentPackageFamily::Governance => {
            push_unique(&mut packages, Some(&deployment.packages.governance_original));
            for id in &deployment.package_history.governance {
                push_unique(&mut packages, Some(id));
            }
            push_unique(&mut packages, Some(&deployment.packages.governance));
        }
        DeploymentPackageFamily::Comments => {
            for id in &deployment.package_history.comments {
                push_unique(&mut packages, Some(id));
            }
            push_unique(&mut packages, Some(&deployment.packages.comments));
        }
        DeploymentPackageFamily::Publishing => {
            push_unique(
                &mut packages,
                deployment.packages.publishing_original.as_deref(),
            );
            for id in &deployment.package_history.publishing {
                push_unique(&mut packages, Some(id));
            }
            push_unique(&mut packages, Some(&deployment.packages.publishing));
        }
        DeploymentPackageFamily::Controller => {
            for id in &deployment.package_history.controller {
                push_unique(&mut packages, Some(id));
            }
            push_unique(&mut packages, deployment.packages.controller.as_deref());
        }
    }
    packages
}
