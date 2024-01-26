const { ethers } = require('hardhat');
const { expect } = require('chai');

const { borshify, borshifyInitialValidators } = require('rainbow-bridge-utils')


describe('New tests', () => {
    let Ed25519;
    let NearBridge;
    let AdminWallet;

    beforeEach(async function () {
        Ed25519 = await (await ethers.getContractFactory('Ed25519')).deploy();
        [AdminWallet] = await ethers.getSigners();
        NearBridge = await (await ethers.getContractFactory('NearBridge')).deploy(
            Ed25519.address,
            ethers.BigNumber.from("1000000000000000000"), // 1e18
            ethers.BigNumber.from("10"), // lock duration
            ethers.BigNumber.from("20000000000"), // replace duration
            await AdminWallet.getAddress(),
            0
        );
        await NearBridge.deposit({ value: ethers.utils.parseEther('1') });
    });

    it('Verify init with validators V1', async function () {
        const initialValidators = borshifyInitialValidators(require('./block_validators_v1_testnet.json').next_bps);

        expect(await NearBridge.initWithValidators(borshifyInitialValidators(require('./block_validators_v1_testnet.json').next_bps)));
    });
});


describe('NearBridge with admin access', () => {
    const ADMIN_ADDRESS_SLOT = 0;
    const UNPAUSED_ALL = 0;
    const PAUSED_DEPOSIT = 1 << 0;
    const PAUSED_WITHDRAW = 1 << 1;
    const PAUSED_ADD_BLOCK = 1 << 2;
    const PAUSED_CHALLENGE = 1 << 3;
    const PAUSED_VERIFY = 1 << 4;

    let nearBridge;
    let adminAccount;
    let userAccount1;
    let userAccount2;
    let newAdminAccount;

    beforeEach(async () => {
        ed25519 = await (await ethers.getContractFactory('Ed25519')).deploy();
        [deployerAccount, userAccount1, userAccount2, newAdminAccount] = await ethers.getSigners();

        // Make the deployer admin
        adminAccount = deployerAccount;

        nearBridge = await (await ethers.getContractFactory('NearBridge')).deploy(
            ed25519.address,
            ethers.BigNumber.from("1000000000000000000"), // 1e18
            ethers.BigNumber.from("10"), // lock duration
            ethers.BigNumber.from("20000000000"), // replace duration
            await adminAccount.getAddress(),
            0
        );
    });

    describe('AdminControlled', async () => {
        it('Admin account matches', async () => {
            expect(
                await nearBridge.admin()
            )
                .to
                .be
                .equal(adminAccount.address);
        });

        it('regular user can not perform admin functions', async () => {
            const recipientBalanceBefore = ethers.BigNumber.from(await ethers.provider.getBalance(userAccount2.address));
            const contractBalanceBefore = ethers.BigNumber.from(await ethers.provider.getBalance(nearBridge.address));

            const amountToTransfer = 4000; // wei
            // user1 tries to perform `adminSendEth()` to replenish user2 balance
            await expect(
                nearBridge
                    .connect(userAccount1)
                    .adminSendEth(userAccount2.address, amountToTransfer)
            )
                .to
                .be
                .reverted;

            const recipientBalanceAfter = ethers.BigNumber.from(await ethers.provider.getBalance(userAccount2.address));
            const contractBalanceAfter = ethers.BigNumber.from(await ethers.provider.getBalance(nearBridge.address));

            expect(recipientBalanceAfter)
                .to
                .be
                .equal(recipientBalanceBefore);
            expect(contractBalanceAfter)
                .to
                .be
                .equal(contractBalanceBefore);

            // Try to pause and unpause
            await expect(nearBridge.connect(userAccount1).adminPause(PAUSED_DEPOSIT)).to.be.reverted;
            await expect(nearBridge.connect(userAccount1).adminPause(PAUSED_WITHDRAW)).to.be.reverted;
            await expect(nearBridge.connect(userAccount1).adminPause(PAUSED_ADD_BLOCK)).to.be.reverted;
            await expect(nearBridge.connect(userAccount1).adminPause(PAUSED_CHALLENGE)).to.be.reverted;
            await expect(nearBridge.connect(userAccount1).adminPause(PAUSED_VERIFY)).to.be.reverted;
            await expect(nearBridge.connect(userAccount1).adminPause(UNPAUSED_ALL)).to.be.reverted;
            // ------------------------------------------

            // Try to use adminSstore
            await expect(
                nearBridge
                    .connect(userAccount1)
                    .adminSstore(0, 1)
            )
                .to
                .be
                .reverted;

            // Try to use adminSstoreWithMask
            await expect(
                nearBridge
                    .connect(userAccount1)
                    .adminSstoreWithMask(0, 1, ethers.BigNumber.from('0x0000ffff'))
            )
                .to
                .be
                .reverted;

            await expect(
                nearBridge
                    .connect(userAccount1)
                    .adminDelegatecall(ethers.constants.AddressZero, ethers.utils.arrayify("0xabcdcafe"))
            )
                .to
                .be
                .reverted;
        });

        it('admin receive eth and transfer eth', async () => {
            const replenishBalanceValue = 1_500_000;

            const options = { value: replenishBalanceValue };
            await nearBridge
                .connect(adminAccount)
                .adminReceiveEth(options);

            const recipientBalanceBefore = ethers.BigNumber.from(await ethers.provider.getBalance(userAccount2.address));
            const contractBalanceBefore = ethers.BigNumber.from(await ethers.provider.getBalance(nearBridge.address));

            // Check the contract has the specified balance available
            expect(contractBalanceBefore)
                .to
                .be
                .equal(replenishBalanceValue);

            // Send eth using admin access
            const amountToTransfer = 4000; // wei
            await nearBridge
                .connect(adminAccount)
                .adminSendEth(userAccount2.address, amountToTransfer);

            const recipientBalanceAfter = ethers.BigNumber.from(await ethers.provider.getBalance(userAccount2.address));
            const contractBalanceAfter = ethers.BigNumber.from(await ethers.provider.getBalance(nearBridge.address));

            expect(recipientBalanceAfter)
                .to
                .be
                .equal(recipientBalanceBefore.add(amountToTransfer));
            expect(contractBalanceAfter)
                .to
                .be
                .equal(contractBalanceBefore.sub(amountToTransfer));
        });

        it('should upgrade the admin address from the provided hex string using (using `adminSstoreWithMask()`)', async () => {
            const initialAdminAddress = await nearBridge.admin();
            expect(initialAdminAddress)
                .to
                .equal(await adminAccount.address);

            const newAdminAddress = '0x0123456789abcdefcafedeadbeefbea77a1de456';
            expect(newAdminAddress)
                .not
                .equal(initialAdminAddress);

            // Mask matches only on the latest 20 bytes (to store the address)
            const mask = ethers.BigNumber.from("0x000000000000000000000000ffffffffffffffffffffffffffffffffffffffff");
            await nearBridge.adminSstoreWithMask(ADMIN_ADDRESS_SLOT, newAdminAddress, mask);

            expect((await nearBridge.admin()).toLowerCase())
                .to
                .equal(newAdminAddress);
        });

        it('should nominate and accept the new admin', async () => {
            const initialAdminAddress = await nearBridge.admin();
            const newAdminAddress = newAdminAccount.address;
            expect(newAdminAddress)
                .not
                .equal(initialAdminAddress);

            await nearBridge.nominateAdmin(newAdminAddress);
            expect(await nearBridge.admin())
                .to
                .equal(initialAdminAddress);
            expect(await nearBridge.nominatedAdmin())
                .to
                .equal(newAdminAddress);

            await nearBridge
                .connect(newAdminAccount)
                .acceptAdmin();
            expect(await nearBridge.admin())
                .to
                .equal(newAdminAddress);
            expect(await nearBridge.nominatedAdmin())
                .to
                .equal(ethers.constants.AddressZero);
        });

        it('should not accept the new admin twice', async () => {
            const initialAdminAddress = await nearBridge.admin();
            const newAdmin = newAdminAccount;
            expect(newAdmin.address)
                .not
                .equal(initialAdminAddress);

            await nearBridge.nominateAdmin(newAdmin.address);
            await nearBridge
                .connect(newAdmin)
                .acceptAdmin();

            expect(await nearBridge.admin())
                .to
                .equal(newAdmin.address);
            await expect(
                nearBridge
                    .connect(newAdmin)
                    .acceptAdmin()
            )
                .to
                .be
                .revertedWith('Nominated admin shouldn\'t be zero address');
        });

        it('should not nominate the same admin', async () => {
            const initialAdminAddress = await nearBridge.admin();
            await expect(nearBridge.nominateAdmin(initialAdminAddress))
                .to
                .be
                .revertedWith('Nominated admin is the same as the current');
        });

        it('should not nominate zero address as an admin', async () => {
            await expect(nearBridge.nominateAdmin(ethers.constants.AddressZero))
                .to
                .be
                .revertedWith('Nominated admin shouldn\'t be zero address');
        });

        it('should not accept zero address as an admin in constructor', async () => {
            await (await ethers.getContractFactory('AdminControlled')).deploy(
                ethers.constants.AddressZero, // admin address
                0, // paused flags
            ).then(() => { throw 'should not allow zero address for admin in constructor' })
                .catch(() => {/* we are good */ })

        });

        it('should reject the nominated admin', async () => {
            const newAdminAddress = '0x0123456789abcdefcafedeadbeefbea77a1de456';
            await nearBridge.nominateAdmin(newAdminAddress);
            expect((await nearBridge.nominatedAdmin()).toLowerCase())
                .to
                .equal(newAdminAddress);

            await nearBridge.rejectNominatedAdmin();
            expect(await nearBridge.nominatedAdmin())
                .to
                .equal(ethers.constants.AddressZero);
        });

        it('should not allow accepting admin any other account than the nominated one', async () => {
            const initialAdminAddress = await nearBridge.admin();
            const newAdminAddress = newAdminAccount.address;
            expect(newAdminAddress)
                .not
                .equal(initialAdminAddress);

            await nearBridge.nominateAdmin(newAdminAddress);

            // Should now allow the current admin to accept the new admin
            await expect(nearBridge
                .connect(adminAccount)
                .acceptAdmin())
                .to
                .be
                .revertedWith('Caller must be the nominated admin');
        });

        it('should not accept the same admin', async () => {
            const initialAdminAddress = await nearBridge.admin();
            // Manually set the nominated admin to the same one
            await nearBridge
                .connect(adminAccount)
                .adminSstore(1, initialAdminAddress);

            // Verify that the nominated admin is indeed the same one (manually set)
            expect(await nearBridge.nominatedAdmin())
                .to
                .equal(initialAdminAddress);

            // Expect to fail in case the nominated admin is the same as the current one
            await expect(nearBridge.acceptAdmin())
                .to
                .be
                .revertedWith('Nominated admin is the same as the current');
        });

        it('should not accept the zero address admin', async () => {
            // Manually set the nominated admin to the same one
            await nearBridge
                .connect(adminAccount)
                .adminSstore(1, ethers.constants.AddressZero);

            // Verify that the nominated admin is indeed the zero address (manually set)
            expect(await nearBridge.nominatedAdmin())
                .to
                .equal(ethers.constants.AddressZero);

            // Expect to fail in case the nominated admin is zero address
            await expect(nearBridge.acceptAdmin())
                .to
                .be
                .revertedWith('Nominated admin shouldn\'t be zero address');
        });
    });
});
