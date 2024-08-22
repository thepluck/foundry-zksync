// SPDX-License-Identifier: UNLICENSED

pragma solidity >=0.8.7 <0.9.0;

import 'ds-test/test.sol';

library BitManipulator {
    struct Input { uint256 data; }

    function setEvenBit(Input storage self, uint256 idx, bool set) internal {
        require(idx < 128, "Invalid index");
        unchecked {
            uint256 bit = 1 << (idx << 1);
            if (set) {
                self.data |= bit;
            } else {
                self.data &= ~bit;
            }
        }
    }
}

contract Helper {
    BitManipulator.Input data = BitManipulator.Input({data: 0});

    function setData(uint256 _data) external {
        data.data = _data;
    }

    function getData() external view returns (uint256) {
        return data.data;
    }

    function setFifth(bool _set) external {
        BitManipulator.setEvenBit(data, 2, _set);
    }
}

contract ZkBitManipulationTest is DSTest {
    Helper helper;

    function setUp() external {
        helper = new Helper();
    }

    function testBitClear() external {
        helper.setData(0x3F /* 0b11_1111 */);
        // set the 5th bit to 0
        helper.setFifth(false);
        assertEq(helper.getData(), 0x2F /* 0b10_1111 */);
    }

    function testBitSet() external {
        helper.setData(0);
        // set the 5th bit to 1
        helper.setFifth(true);
        assertEq(helper.getData(), 0x10 /* 0b01_0000 */);
    }
}
