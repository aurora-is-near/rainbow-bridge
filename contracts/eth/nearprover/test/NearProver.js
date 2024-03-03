const { expect } = require('chai');
const { ethers } = require('hardhat');
const { borshifyOutcomeProof } = require(`rainbow-bridge-utils`);
const fs = require('fs').promises;
const { computeMerkleRoot } = require('../utils/utils');
const bs58 = require('bs58');
const { getAllJsonFilesRecursive } = require('../../../../utils/proof-vector-utils');

let NearProver, NearBridgeMock;

beforeEach(async function () {
    NearBridgeMock = await (await ethers.getContractFactory('NearBridgeMock')).deploy();
    NearProver = await (await ethers.getContractFactory('NearProver')).deploy(
        NearBridgeMock.address,
        ethers.constants.AddressZero,
        0
    );
});

async function runTestVectors(testVectors) {
    for (const test of testVectors) {
        const {
            description,
            params: { proof, block_merkle_root },
            expected: { is_valid, error },
        } = test;
        let wasValid;
        let executionError;
        try {
            const blockMerkleRoot = bs58.decode(block_merkle_root).toString('hex');
            // Note, the height shouldn't matter here, defaulting to 100
            const height = 100;
            expect(blockMerkleRoot === computeMerkleRoot(proof).toString('hex')).to.be.true;
            await NearBridgeMock.setBlockMerkleRoot(height, "0x" + blockMerkleRoot);
            wasValid = await NearProver.proveOutcome(borshifyOutcomeProof(proof), height);
        } catch (error) {
            wasValid = false;
            executionError = error;
        }
        if (wasValid !== is_valid) {
            const prefix = `Test Case "${description}": FAILED - expected`;
            throw new Error(
                `${prefix} ${is_valid
                    ? `valid, got error ${executionError}`
                    : `invalid result${error ? ` with error "${error}"` : ""}`
                }`
            );
        }
    }
}

describe('execution outcome proof vectors', async function () {
    const files = getAllJsonFilesRecursive("../../../near-light-client-tests/test-vectors/executions");

    for (const file of files) {
        const fileName = file.split('\\').pop().split('/').pop();
        it(`execution vector file "${fileName}"`, async function () {
            await runTestVectors(require("../" + file));
        });
    }
});

async function testProof(merkleRoot, height, proofPath) {
    let proof = require(proofPath);
    console.log(computeMerkleRoot(proof).toString('hex'));
    expect(merkleRoot === '0x' + computeMerkleRoot(proof).toString('hex')).to.be.true;
    proof = borshifyOutcomeProof(proof);
    await NearBridgeMock.setBlockMerkleRoot(height, merkleRoot);
    expect(await NearProver.proveOutcome(proof, height)).to.be.true;
}

it('should be ok', async function () {
    await testProof('0x22f00dd154366d758cd3e4fe81c1caed8e0db6227fe4b2b52a8e5a468aa0a723', 498, './proof2.json');
    await testProof('0x0d0776820a9a81481a559c36fd5d69c33718fb7d7fd3be7564a446e043e2cb35', 1705, './proof3.json');
    await testProof('0x1f7129496c461c058fb3daf258d89bf7dacb4efad5742351f66098a00bb6fa53', 5563, './proof4.json');
    await testProof('0xa9cd8eb4dd92ba5f2fef47d68e1d73ac8c57047959f6f8a2dcc664419e74e4b8', 384, './proof5.json');
    await testProof('0xcc3954a51b7c1a86861df8809f79c2bf839741e3e380e28360b8b3970a5d90bd', 377, './proof6.json');
    await testProof('0x8298c9cd1048df03e9ccefac4b022636a30a2f7e6a8c33cc4104901b92e08dfd', 93700915, './proof7.json');
});

if (process.env['NEAR_PROOFS_DIR']) {
    it('should able to verify proofs from dump', async function () {
        this.timeout(0);
        let proofFiles = await fs.readdir(process.env['NEAR_PROOFS_DIR']);

        for (let i = 0; i < proofFiles.length; i++) {
            let proof = require(process.env['NEAR_PROOFS_DIR'] + '/' + proofFiles[i]);
            const height = proof.block_header_lite.inner_lite.height;
            await NearBridgeMock.setBlockMerkleRoot(height, '0x' + computeMerkleRoot(proof).toString('hex'));
            proof = borshifyOutcomeProof(proof);
            expect(await NearProver.proveOutcome(proof, height)).to.be.true;
            console.log('proved proof ' + proofFiles[i]);
        }
    });
}

describe('NearProver with admin access', () => {
    const BRIDGE_ADDRESS_SLOT = 3;

    beforeEach(async () => {
        [deployerAccount] = await ethers.getSigners();

        // Make the deployer admin
        adminAccount = deployerAccount;

        nearBridgeMock = await (await ethers.getContractFactory('NearBridgeMock')).deploy();
        nearProver = await (await ethers.getContractFactory('NearProver')).deploy(
            nearBridgeMock.address,
            adminAccount.address,
            0
        );

    });

    describe('Upgradability', async () => {
        it('should upgrade the bridge address', async () => {
            const initialBridgeAddress = await nearProver.bridge();
            expect(initialBridgeAddress)
                .to
                .equal(nearBridgeMock.address);

            const newBridge = await (await ethers.getContractFactory('NearBridgeMock')).deploy();
            expect(await newBridge.address)
                .to
                .not
                .equal(initialBridgeAddress);

            // Mask matches only on the latest 20 bytes (to store the address)
            const mask = ethers.BigNumber.from("0x000000000000000000000000ffffffffffffffffffffffffffffffffffffffff");
            nearProver.adminSstoreWithMask(BRIDGE_ADDRESS_SLOT, newBridge.address, mask);

            expect(await nearProver.bridge())
                .to
                .equal(newBridge.address);
        });

        it('should upgrade the bridge address from the provided hex string', async () => {
            const initialBridgeAddress = await nearProver.bridge();
            expect(initialBridgeAddress)
                .to
                .equal(nearBridgeMock.address);

            const newBridgeAddress = '0x891B2749238B27fF58e951088e55b04de71Dc374';
            const newBridgeAddressBN = ethers.BigNumber.from(newBridgeAddress);

            const mask = ethers.BigNumber.from("0x000000000000000000000000ffffffffffffffffffffffffffffffffffffffff");
            nearProver.adminSstoreWithMask(BRIDGE_ADDRESS_SLOT, newBridgeAddressBN, mask);

            expect(await nearProver.bridge())
                .to
                .equal(newBridgeAddress);
        });
    });
});
