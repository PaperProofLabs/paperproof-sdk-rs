// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use std::process::Command;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    deployment::Deployment,
    error::{PaperProofError, Result},
    events::{
        AddVersionResult, CommentResult, LikeResult, ProposalExecutedResult,
        ProposalFinalizedResult, ProposalResult, PublishResult, SuiEventEnvelope, VoteCastResult,
    },
    providers::{
        BuiltTransaction, PaperProofExecutionProvider, ProviderExecutionOptions,
        ProviderExecutionOutput,
    },
    transaction::{MoveArgument, MoveCall, TransactionPlan},
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ExecutionMode {
    Preview,
    DryRun,
    DevInspect,
    Execute,
}

impl From<&ProviderExecutionOptions> for CliExecutionOptions {
    fn from(value: &ProviderExecutionOptions) -> Self {
        Self {
            sui_binary: "sui".to_string(),
            sender: value.sender.clone(),
            gas_budget: value.gas_budget,
            gas_coin: value.gas_coin.clone(),
            mode: value.mode.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CliExecutionOptions {
    pub sui_binary: String,
    pub sender: Option<String>,
    pub gas_budget: Option<u64>,
    pub gas_coin: Option<String>,
    pub mode: ExecutionMode,
}

impl Default for CliExecutionOptions {
    fn default() -> Self {
        Self {
            sui_binary: "sui".to_string(),
            sender: None,
            gas_budget: None,
            gas_coin: None,
            mode: ExecutionMode::Preview,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CliExecutionOutput {
    pub status_success: bool,
    pub digest: Option<String>,
    pub raw_stdout: String,
    pub raw_stderr: String,
    pub json: Option<Value>,
}

impl CliExecutionOutput {
    pub fn events(&self) -> Result<Vec<SuiEventEnvelope>> {
        let Some(json) = &self.json else {
            return Ok(Vec::new());
        };
        crate::events::events_from_value(json)
    }

    pub fn publish_result(&self, deployment: &Deployment) -> Result<PublishResult> {
        crate::events::extract_publish_result(&self.events()?, Some(deployment))
    }

    pub fn add_version_result(&self, deployment: &Deployment) -> Result<AddVersionResult> {
        crate::events::extract_add_version_result(&self.events()?, Some(deployment))
    }

    pub fn comment_result(&self, deployment: &Deployment) -> Result<CommentResult> {
        crate::events::extract_comment_result(&self.events()?, Some(deployment))
    }

    pub fn proposal_result(&self, deployment: &Deployment) -> Result<ProposalResult> {
        crate::events::extract_proposal_result(&self.events()?, Some(deployment))
    }

    pub fn like_result(&self, deployment: &Deployment) -> Result<Option<LikeResult>> {
        crate::events::extract_like_result(&self.events()?, Some(deployment))
    }

    pub fn unlike_result(&self, deployment: &Deployment) -> Result<Option<LikeResult>> {
        crate::events::extract_unlike_result(&self.events()?, Some(deployment))
    }

    pub fn vote_cast_results(&self, deployment: &Deployment) -> Result<Vec<VoteCastResult>> {
        crate::events::extract_vote_cast_results(&self.events()?, Some(deployment))
    }

    pub fn proposal_finalized_result(
        &self,
        deployment: &Deployment,
    ) -> Result<Option<ProposalFinalizedResult>> {
        crate::events::extract_proposal_finalized_result(&self.events()?, Some(deployment))
    }

    pub fn proposal_executed_result(
        &self,
        deployment: &Deployment,
    ) -> Result<Option<ProposalExecutedResult>> {
        crate::events::extract_proposal_executed_result(&self.events()?, Some(deployment))
    }

    pub fn proposal_expired_result(
        &self,
        deployment: &Deployment,
    ) -> Result<Option<crate::events::ProposalExpiredResult>> {
        crate::events::extract_proposal_expired_result(&self.events()?, Some(deployment))
    }

    pub fn vote_claimed_result(
        &self,
        deployment: &Deployment,
    ) -> Result<Option<crate::events::VoteClaimedResult>> {
        crate::events::extract_vote_claimed_result(&self.events()?, Some(deployment))
    }
}

#[derive(Clone, Debug)]
pub struct SuiCliExecutor {
    deployment: Deployment,
}

impl SuiCliExecutor {
    pub fn new(deployment: Deployment) -> Self {
        Self { deployment }
    }

    pub fn mainnet() -> Self {
        Self::new(crate::deployment::mainnet_deployment())
    }

    pub fn to_cli_args(
        &self,
        plan: &TransactionPlan,
        options: &CliExecutionOptions,
    ) -> Result<Vec<String>> {
        if plan.is_empty() {
            return Err(PaperProofError::TransactionBuild {
                message: "transaction plan must contain at least one Move call".to_string(),
            });
        }
        let mut args = vec!["client".to_string(), "ptb".to_string()];
        let mut temp_counter = 0usize;
        for call in &plan.calls {
            self.append_move_call(&mut args, call, &mut temp_counter)?;
        }
        if let Some(sender) = &options.sender {
            crate::validation::validate_address(sender)?;
            args.extend(["--sender".to_string(), format!("@{sender}")]);
        }
        if let Some(gas_budget) = options.gas_budget {
            args.extend(["--gas-budget".to_string(), gas_budget.to_string()]);
        }
        if let Some(gas_coin) = &options.gas_coin {
            crate::validation::validate_object_id(gas_coin)?;
            args.extend(["--gas-coin".to_string(), gas_coin.clone()]);
        }
        match options.mode {
            ExecutionMode::Preview => args.push("--preview".to_string()),
            ExecutionMode::DryRun => args.push("--dry-run".to_string()),
            ExecutionMode::DevInspect => args.push("--dev-inspect".to_string()),
            ExecutionMode::Execute => {}
        }
        args.push("--json".to_string());
        Ok(args)
    }

    pub fn run(
        &self,
        plan: &TransactionPlan,
        options: &CliExecutionOptions,
    ) -> Result<CliExecutionOutput> {
        let args = self.to_cli_args(plan, options)?;
        let output = Command::new(&options.sui_binary)
            .args(args)
            .output()
            .map_err(|err| PaperProofError::TransactionExecution {
                message: format!("failed to start Sui CLI: {err}"),
            })?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let json = serde_json::from_str::<Value>(&stdout).ok();
        let digest = json.as_ref().and_then(extract_digest);
        if !output.status.success() {
            return Err(PaperProofError::TransactionExecution {
                message: format!(
                    "Sui CLI exited with status {}. stderr: {} stdout: {}",
                    output.status, stderr, stdout
                ),
            });
        }
        Ok(CliExecutionOutput {
            status_success: infer_success(&json),
            digest,
            raw_stdout: stdout,
            raw_stderr: stderr,
            json,
        })
    }

    fn append_move_call(
        &self,
        args: &mut Vec<String>,
        call: &MoveCall,
        temp_counter: &mut usize,
    ) -> Result<()> {
        let mut function_args = Vec::new();
        for argument in &call.arguments {
            function_args.push(self.render_argument(args, argument, temp_counter)?);
        }
        args.extend([
            "--move-call".to_string(),
            call.target.clone(),
            "".to_string(),
            function_args.join(" "),
        ]);
        Ok(())
    }

    fn render_argument(
        &self,
        args: &mut Vec<String>,
        argument: &MoveArgument,
        temp_counter: &mut usize,
    ) -> Result<String> {
        match argument {
            MoveArgument::Object(value) => {
                crate::validation::validate_object_id(value)?;
                Ok(format!("@{value}"))
            }
            MoveArgument::Address(value) => {
                crate::validation::validate_address(value)?;
                Ok(format!("@{value}"))
            }
            MoveArgument::String(value) => Ok(shell_json(value)?),
            MoveArgument::U8(value) => Ok(value.to_string()),
            MoveArgument::U64(value) => Ok(value.to_string()),
            MoveArgument::Bool(value) => Ok(value.to_string()),
            MoveArgument::Bytes(value) => self.u8_vector_arg(args, value, temp_counter),
            MoveArgument::StringVector(values) => {
                self.string_vector_arg(args, values, temp_counter)
            }
            MoveArgument::MetadataVector(values) => {
                self.metadata_vector_arg(args, values, temp_counter)
            }
            MoveArgument::OptionalObject(value) => {
                self.optional_sui_coin_arg(args, value.as_deref(), temp_counter)
            }
            MoveArgument::OptionalAddress(value) => {
                self.optional_address_arg(args, value.as_deref(), temp_counter)
            }
            MoveArgument::OptionalObjectId(value) => {
                self.optional_object_id_arg(args, value.as_deref(), temp_counter)
            }
        }
    }

    fn metadata_vector_arg(
        &self,
        args: &mut Vec<String>,
        values: &[crate::types::MetadataAttribute],
        temp_counter: &mut usize,
    ) -> Result<String> {
        let mut names = Vec::new();
        for value in values {
            let name = next_temp("meta", temp_counter);
            args.extend([
                "--move-call".to_string(),
                format!(
                    "{}::publishing::metadata_attribute",
                    self.deployment.packages.publishing
                ),
                "".to_string(),
                format!("{} {}", shell_json(&value.key)?, shell_json(&value.value)?),
                "--assign".to_string(),
                name.clone(),
            ]);
            names.push(name);
        }
        let vec_name = next_temp("metadata_vec", temp_counter);
        args.extend([
            "--make-move-vec".to_string(),
            format!(
                "<{}::publishing::MetadataAttribute>",
                self.deployment.packages.publishing
            ),
            format!("[{}]", names.join(",")),
            "--assign".to_string(),
            vec_name.clone(),
        ]);
        Ok(vec_name)
    }

    fn string_vector_arg(
        &self,
        args: &mut Vec<String>,
        values: &[String],
        temp_counter: &mut usize,
    ) -> Result<String> {
        let vec_name = next_temp("string_vec", temp_counter);
        let items = values
            .iter()
            .map(|value| shell_json(value))
            .collect::<Result<Vec<_>>>()?;
        args.extend([
            "--make-move-vec".to_string(),
            "<0x1::string::String>".to_string(),
            format!("[{}]", items.join(",")),
            "--assign".to_string(),
            vec_name.clone(),
        ]);
        Ok(vec_name)
    }

    fn u8_vector_arg(
        &self,
        args: &mut Vec<String>,
        values: &[u8],
        temp_counter: &mut usize,
    ) -> Result<String> {
        let vec_name = next_temp("u8_vec", temp_counter);
        let items = values
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(",");
        args.extend([
            "--make-move-vec".to_string(),
            "<u8>".to_string(),
            format!("[{items}]"),
            "--assign".to_string(),
            vec_name.clone(),
        ]);
        Ok(vec_name)
    }

    fn optional_sui_coin_arg(
        &self,
        args: &mut Vec<String>,
        value: Option<&str>,
        temp_counter: &mut usize,
    ) -> Result<String> {
        let name = next_temp("sui_payment", temp_counter);
        let function = if value.is_some() {
            "0x1::option::some"
        } else {
            "0x1::option::none"
        };
        let mut function_args = String::new();
        if let Some(object_id) = value {
            crate::validation::validate_object_id(object_id)?;
            function_args = format!("@{object_id}");
        }
        args.extend([
            "--move-call".to_string(),
            function.to_string(),
            "<0x2::coin::Coin<0x2::sui::SUI>>".to_string(),
            function_args,
            "--assign".to_string(),
            name.clone(),
        ]);
        Ok(name)
    }

    fn optional_address_arg(
        &self,
        args: &mut Vec<String>,
        value: Option<&str>,
        temp_counter: &mut usize,
    ) -> Result<String> {
        let name = next_temp("address_option", temp_counter);
        let function = if value.is_some() {
            "0x1::option::some"
        } else {
            "0x1::option::none"
        };
        let mut function_args = String::new();
        if let Some(address) = value {
            crate::validation::validate_address(address)?;
            function_args = format!("@{address}");
        }
        args.extend([
            "--move-call".to_string(),
            function.to_string(),
            "<0x2::object::ID>".to_string(),
            function_args,
            "--assign".to_string(),
            name.clone(),
        ]);
        Ok(name)
    }

    fn optional_object_id_arg(
        &self,
        args: &mut Vec<String>,
        value: Option<&str>,
        temp_counter: &mut usize,
    ) -> Result<String> {
        let name = next_temp("object_id_option", temp_counter);
        let function = if value.is_some() {
            "0x1::option::some"
        } else {
            "0x1::option::none"
        };
        let mut function_args = String::new();
        if let Some(object_id) = value {
            crate::validation::validate_object_id(object_id)?;
            function_args = format!("@{object_id}");
        }
        args.extend([
            "--move-call".to_string(),
            function.to_string(),
            "<address>".to_string(),
            function_args,
            "--assign".to_string(),
            name.clone(),
        ]);
        Ok(name)
    }
}

#[async_trait]
impl PaperProofExecutionProvider for SuiCliExecutor {
    async fn build_transaction(
        &self,
        plan: &TransactionPlan,
        options: &ProviderExecutionOptions,
    ) -> Result<BuiltTransaction> {
        let cli_options = CliExecutionOptions::from(options);
        self.to_cli_args(plan, &cli_options)
            .map(BuiltTransaction::SuiCliArgs)
    }

    async fn dry_run(
        &self,
        plan: &TransactionPlan,
        options: &ProviderExecutionOptions,
    ) -> Result<ProviderExecutionOutput> {
        let mut cli_options = CliExecutionOptions::from(options);
        cli_options.mode = ExecutionMode::DryRun;
        self.run(plan, &cli_options)
            .map(ProviderExecutionOutput::SuiCli)
    }

    async fn dev_inspect(
        &self,
        plan: &TransactionPlan,
        options: &ProviderExecutionOptions,
    ) -> Result<ProviderExecutionOutput> {
        let mut cli_options = CliExecutionOptions::from(options);
        cli_options.mode = ExecutionMode::DevInspect;
        self.run(plan, &cli_options)
            .map(ProviderExecutionOutput::SuiCli)
    }

    async fn sign_and_execute(
        &self,
        plan: &TransactionPlan,
        options: &ProviderExecutionOptions,
    ) -> Result<ProviderExecutionOutput> {
        let mut cli_options = CliExecutionOptions::from(options);
        cli_options.mode = ExecutionMode::Execute;
        self.run(plan, &cli_options)
            .map(ProviderExecutionOutput::SuiCli)
    }
}

fn next_temp(prefix: &str, counter: &mut usize) -> String {
    let name = format!("{prefix}_{counter}");
    *counter += 1;
    name
}

fn shell_json(value: &str) -> Result<String> {
    serde_json::to_string(value).map_err(Into::into)
}

fn extract_digest(value: &Value) -> Option<String> {
    value
        .get("digest")
        .or_else(|| value.pointer("/effects/transactionDigest"))
        .or_else(|| value.pointer("/effects/certificate/digest"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn infer_success(value: &Option<Value>) -> bool {
    value
        .as_ref()
        .and_then(|json| {
            json.pointer("/effects/status/status")
                .or_else(|| json.pointer("/status/status"))
                .and_then(Value::as_str)
        })
        .map(|status| status.eq_ignore_ascii_case("success"))
        .unwrap_or(true)
}
