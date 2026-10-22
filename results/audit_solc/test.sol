// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract Vault {
    uint256 constant MAX = 100;
    mapping(address => uint256) private balances;
    uint256 public total;

    function deposit(uint256 amount) public {
        require(amount <= MAX, "too much");
        uint256 newBal = balances[msg.sender] + amount;
        balances[msg.sender] = newBal;
        total += amount;
    }
}
