//! Forge tests for basic zkysnc functionality.

use crate::{config::*, test_helpers::TEST_DATA_DEFAULT};
use forge::revm::primitives::SpecId;
use foundry_test_utils::Filter;

#[tokio::test(flavor = "multi_thread")]
async fn test_zk_block_information_is_consistent() {
    let runner = TEST_DATA_DEFAULT.runner_zksync();
    let filter =
        Filter::new("testZkBasicBlockNumber|testZkBasicBlockTimestamp", "ZkBasicTest", ".*");

    TestConfig::with_filter(runner, filter).evm_spec(SpecId::SHANGHAI).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_zk_address_balance_is_consistent() {
    let runner = TEST_DATA_DEFAULT.runner_zksync();
    let filter = Filter::new("testZkBasicAddressBalance", "ZkBasicTest", ".*");

    TestConfig::with_filter(runner, filter).evm_spec(SpecId::SHANGHAI).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_zk_propagated_block_env_is_consistent() {
    let runner = TEST_DATA_DEFAULT.runner_zksync();
    let filter = Filter::new(
        "testZkPropagatedBlockEnv|testZkBasicBlockBaseFee|testZkBlockHashWithNewerBlocks",
        "ZkBasicTest",
        ".*",
    );

    TestConfig::with_filter(runner, filter).evm_spec(SpecId::SHANGHAI).run().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_zk_bit_manipulation() {
    let mut evm_with_zk_tests_config = TEST_DATA_DEFAULT.zk_test_data.zk_config.clone();
    evm_with_zk_tests_config.zksync.compile = false;
    evm_with_zk_tests_config.zksync.startup = false;

    let runner = TEST_DATA_DEFAULT.runner_with_zksync_config(evm_with_zk_tests_config);
    let runner_zk = TEST_DATA_DEFAULT.runner_zksync();

    let mut zk_test = TestConfig::with_filter(runner_zk, Filter::contract("ZkBitManipulationTest")).evm_spec(SpecId::SHANGHAI);
    let mut evm_test = TestConfig::with_filter(runner, Filter::contract("ZkBitManipulationTest")).evm_spec(SpecId::SHANGHAI);

    tokio::join!(evm_test.run(), zk_test.run());
}
