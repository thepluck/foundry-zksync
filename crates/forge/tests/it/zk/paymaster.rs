//! Forge tests for zksync contracts.

use foundry_config::fs_permissions::PathPermission;
use foundry_test_utils::util;

use std::fs;

#[tokio::test(flavor = "multi_thread")]
async fn test_zk_contract_paymaster() {
    let (prj, mut cmd) = util::setup_forge(
        "test_zk_contract_paymaster",
        foundry_test_utils::foundry_compilers::PathStyle::Dapptools,
    );
    util::initialize(prj.root());

    cmd.args([
        "install",
        "OpenZeppelin/openzeppelin-contracts",
        "matter-labs/era-contracts",
        "--no-commit",
        "--shallow",
    ])
    .ensure_execute_success()
    .expect("able to install dependencies");

    //ls the lib/era-contracts/system-contracts folder
    let entries = fs::read_dir(prj.root().join("lib").join("era-contracts").join("system-contracts").join("contracts").join("interfaces")).unwrap();
    for entry in entries {
        println!("Found in lib: {:?}", entry.unwrap().path().to_str().unwrap());
    }

    cmd.forge_fuse();

    let mut config = cmd.config();
    config.fs_permissions.add(PathPermission::read("./zkout"));
    prj.write_config(config);

    prj.add_source("MyPaymaster.sol", include_str!("../../fixtures/zk/MyPaymaster.sol")).unwrap();
    prj.add_source("Paymaster.t.sol", include_str!("../../fixtures/zk/Paymaster.t.sol")).unwrap();



    //add mc to the command
    cmd.args(["test", "--zk-startup", "--evm-version", "shanghai", "--via-ir"]);
    assert!(cmd.stdout_lossy().contains("Suite result: ok"));
}
