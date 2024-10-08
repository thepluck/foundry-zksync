use crate::{build::LinkedBuildData, sequence::ScriptSequenceKind, ScriptArgs, ScriptConfig};
use alloy_primitives::{hex, Address};
use eyre::Result;
use forge_verify::{RetryArgs, VerifierArgs, VerifyArgs};
use foundry_cli::opts::{EtherscanOpts, ProjectPathsArgs};
use foundry_common::ContractsByArtifact;
use foundry_compilers::{info::ContractInfo, Project};
use foundry_config::{Chain, Config};
use foundry_zksync_compiler::ZKSYNC_ARTIFACTS_DIR;
use semver::Version;

/// State after we have broadcasted the script.
/// It is assumed that at this point [BroadcastedState::sequence] contains receipts for all
/// broadcasted transactions.
pub struct BroadcastedState {
    pub args: ScriptArgs,
    pub script_config: ScriptConfig,
    pub build_data: LinkedBuildData,
    pub sequence: ScriptSequenceKind,
}

impl BroadcastedState {
    pub async fn verify(self) -> Result<()> {
        let Self { args, script_config, build_data, mut sequence, .. } = self;

        let verify = VerifyBundle::new(
            &script_config.config.project()?,
            &script_config.config,
            build_data.known_contracts,
            args.retry,
            args.verifier,
        );

        for sequence in sequence.sequences_mut() {
            sequence.verify_contracts(&script_config.config, verify.clone()).await?;
        }

        Ok(())
    }
}

/// Data struct to help `ScriptSequence` verify contracts on `etherscan`.
#[derive(Clone)]
pub struct VerifyBundle {
    pub num_of_optimizations: Option<usize>,
    pub known_contracts: ContractsByArtifact,
    pub project_paths: ProjectPathsArgs,
    pub etherscan: EtherscanOpts,
    pub retry: RetryArgs,
    pub verifier: VerifierArgs,
    pub via_ir: bool,
    pub zksync: bool,
}

impl VerifyBundle {
    pub fn new(
        project: &Project,
        config: &Config,
        known_contracts: ContractsByArtifact,
        retry: RetryArgs,
        verifier: VerifierArgs,
    ) -> Self {
        let num_of_optimizations =
            if config.optimizer { Some(config.optimizer_runs) } else { None };

        let config_path = config.get_config_path();

        let project_paths = ProjectPathsArgs {
            root: Some(project.paths.root.clone()),
            contracts: Some(project.paths.sources.clone()),
            remappings: project.paths.remappings.clone(),
            remappings_env: None,
            cache_path: Some(project.paths.cache.clone()),
            lib_paths: project.paths.libraries.clone(),
            hardhat: config.profile == Config::HARDHAT_PROFILE,
            config_path: if config_path.exists() { Some(config_path) } else { None },
        };

        let via_ir = config.via_ir;
        let zksync = config.zksync.should_compile();

        Self {
            num_of_optimizations,
            known_contracts,
            etherscan: Default::default(),
            project_paths,
            retry,
            verifier,
            via_ir,
            zksync,
        }
    }

    /// Configures the chain and sets the etherscan key, if available
    pub fn set_chain(&mut self, config: &Config, chain: Chain) {
        // If dealing with multiple chains, we need to be able to change inbetween the config
        // chain_id.
        self.etherscan.key = config.get_etherscan_api_key(Some(chain));
        self.etherscan.chain = Some(chain);
    }

    /// Given a `VerifyBundle` and contract details, it tries to generate a valid `VerifyArgs` to
    /// use against the `contract_address`.
    pub fn get_verify_args(
        &self,
        contract_address: Address,
        create2_offset: usize,
        data: &[u8],
        libraries: &[String],
        zksync: bool,
    ) -> Option<VerifyArgs> {
        for (artifact, contract) in self.known_contracts.iter() {
            let Some(bytecode) = contract.bytecode() else { continue };
            // If it's a CREATE2, the tx.data comes with a 32-byte salt in the beginning
            // of the transaction
            let (contract_match, effective_create2_offset) = if zksync {
                // For zkSync, we need to handle both EVM and zkEVM artifacts
                // as known_contracts may contain both types
                let is_zksync_artifact =
                    artifact.path.to_string_lossy().contains(ZKSYNC_ARTIFACTS_DIR);
                if is_zksync_artifact {
                    // zkSync-specific artifact handling
                    let bytecode_hash =
                        foundry_zksync_core::hash_bytecode(contract.deployed_bytecode().unwrap());

                    // The 36-byte offset consists of:
                    // - 4 bytes: selector
                    // - 32 bytes: salt (for CREATE2)
                    let data_after_offset = &data[4 + 32..];
                    (data_after_offset.starts_with(bytecode_hash.as_bytes()), 36)
                } else {
                    (false, create2_offset)
                }
            } else {
                (data[create2_offset..].starts_with(bytecode), create2_offset)
            };

            if contract_match {
                let constructor_args = if zksync {
                    // For zkSync:
                    // 32 bytes: bytecode hash length
                    // 64 bytes: offset (32 bytes) + constructor arguments length (32 bytes)
                    // The rest: actual constructor arguments
                    data[effective_create2_offset + 32 + 64..].to_vec()
                } else {
                    data[effective_create2_offset + bytecode.len()..].to_vec()
                };

                if artifact.source.extension().map_or(false, |e| e.to_str() == Some("vy")) {
                    warn!("Skipping verification of Vyper contract: {}", artifact.name);
                }

                let contract = ContractInfo {
                    path: Some(artifact.source.to_string_lossy().to_string()),
                    name: artifact.name.clone(),
                };

                // We strip the build metadadata information, since it can lead to
                // etherscan not identifying it correctly. eg:
                // `v0.8.10+commit.fc410830.Linux.gcc` != `v0.8.10+commit.fc410830`
                let version = Version::new(
                    artifact.version.major,
                    artifact.version.minor,
                    artifact.version.patch,
                );

                let verify = VerifyArgs {
                    address: contract_address,
                    contract: Some(contract),
                    compiler_version: Some(version.to_string()),
                    constructor_args: Some(hex::encode(constructor_args)),
                    constructor_args_path: None,
                    num_of_optimizations: self.num_of_optimizations,
                    etherscan: self.etherscan.clone(),
                    rpc: Default::default(),
                    flatten: false,
                    force: false,
                    skip_is_verified_check: true,
                    watch: true,
                    retry: self.retry,
                    libraries: libraries.to_vec(),
                    root: None,
                    verifier: self.verifier.clone(),
                    via_ir: self.via_ir,
                    evm_version: None,
                    show_standard_json_input: false,
                    guess_constructor_args: false,
                    zksync: self.zksync,
                };

                return Some(verify)
            }
        }
        None
    }
}
