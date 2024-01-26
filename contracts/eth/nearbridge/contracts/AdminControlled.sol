// SPDX-License-Identifier: GPL-3.0-or-later
pragma solidity ^0.8;

contract AdminControlled {
    address public admin;
    address public nominatedAdmin;
    uint public paused;

    constructor(address _admin, uint flags) {
        verifyAdminAddress(_admin);
        admin = _admin;
        paused = flags;
    }

    modifier onlyAdmin() {
        require(msg.sender == admin);
        _;
    }

    modifier pausable(uint flag) {
        require((paused & flag) == 0 || msg.sender == admin);
        _;
    }

    function adminPause(uint flags) public onlyAdmin {
        paused = flags;
    }

    function adminSstore(uint key, uint value) public onlyAdmin {
        assembly {
            sstore(key, value)
        }
    }

    function adminSstoreWithMask(
        uint key,
        uint value,
        uint mask
    ) public onlyAdmin {
        assembly {
            let oldval := sload(key)
            sstore(key, xor(and(xor(value, oldval), mask), oldval))
        }
    }

    function verifyAdminAddress(address newAdmin) internal view {
        require(newAdmin != admin, "Nominated admin is the same as the current");
        // Zero address shouldn't be allowed as a security measure.
        // If it's needed to remove the admin consider using address with all "1" digits.
        require(newAdmin != address(0), "Nominated admin shouldn't be zero address");
    }

    function nominateAdmin(address newAdmin) public onlyAdmin {
        verifyAdminAddress(newAdmin);
        nominatedAdmin = newAdmin;
    }

    function acceptAdmin() public {
        verifyAdminAddress(nominatedAdmin);
        // Only nominated admin could accept its admin rights
        require(msg.sender == nominatedAdmin, "Caller must be the nominated admin");

        admin = nominatedAdmin;
        // Explicitly set not allowed zero address for `nominatedAdmin` so it's impossible to accidentally change
        // the admin if calling the function twice
        nominatedAdmin = address(0);
    }

    function rejectNominatedAdmin() public onlyAdmin {
        nominatedAdmin = address(0);
    }

    function adminSendEth(address payable destination, uint amount) public onlyAdmin {
        destination.call{value: amount}("");
    }

    function adminReceiveEth() public payable onlyAdmin {}

    function adminDelegatecall(address target, bytes memory data) public payable onlyAdmin returns (bytes memory) {
        (bool success, bytes memory rdata) = target.delegatecall(data);
        require(success);
        return rdata;
    }
}
