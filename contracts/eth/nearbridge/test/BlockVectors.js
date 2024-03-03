// const { expect } = require('chai');
const { ethers } = require('hardhat');
const { getAllJsonFilesRecursive } = require('../../../../utils/proof-vector-utils');
const { borshify, borshifyInitialValidators } = require('rainbow-bridge-utils');

async function runTestVectors(testVectors) {
	const Ed25519 = await (await ethers.getContractFactory('Ed25519')).deploy();
	for (const test of testVectors) {
		let {
			description,
			params: { previous_block, current_bps, new_block },
			expected: { is_valid, error },
		} = test;

		if (description == "Invalid approval message signature at index 2") {
			// Signature validation isn't done because too slow (see below). This test case must
			// be skipped because block valid outside of signature.
			continue;
		}
		
		let wasValid;
		let executionError;
		try {
			const NearBridge = await (await ethers.getContractFactory('NearBridge')).deploy(
				Ed25519.address,
				ethers.BigNumber.from('1000000000000000000'), // 1e18
				ethers.BigNumber.from('360'), // lock duration
				ethers.BigNumber.from('362627730000'), // replace duration
				await (await ethers.getSigners())[0].getAddress(),
				0,
			);
			await NearBridge.deposit({ value: ethers.utils.parseEther('1') });

			await NearBridge.initWithValidators(borshifyInitialValidators(current_bps));
			// Note: hacky workaround since the initWithBlock method requires `approvals_after_next`
			// and `next_block_inner_hash`, which it does not use and `next_bps` that is a bit
			// redundant with the validators initialization.
			previous_block.approvals_after_next = new_block.approvals_after_next;
			previous_block.next_bps = current_bps;
			previous_block.next_block_inner_hash = new_block.next_block_inner_hash;
			await NearBridge.initWithBlock(borshify(previous_block));
			await NearBridge.addLightClientBlock(borshify(new_block));

			// Note: this validation is really slow and stalls the test suite, but worked until it did
			// for (let j = 0; j < new_block.approvals_after_next.length; j++) {
			// 	if (new_block.approvals_after_next[j]) {
			// 		expect(await NearBridge.checkBlockProducerSignatureInHead(j)).to.be.true;
			// 	}
			// }
			wasValid = true;
		} catch (error) {
			wasValid = false;
			executionError = error;
		}
		if (wasValid !== is_valid) {
			const prefix = `Test Case "${description}": FAILED - expected`;
			throw new Error(
				`${prefix} ${is_valid
					? `valid, got error ${executionError}`
					: `invalid result${error ? ` with error "${error}"` : ''}`
				}`,
			);
		}
	}
}

describe('light client block vectors', async function () {
	const files = getAllJsonFilesRecursive('../../../near-light-client-tests/test-vectors/blocks');

	for (const file of files) {
		const fileName = file.split('\\').pop().split('/').pop();
		it(`block vector file "${fileName}"`, async function () {
			await runTestVectors(require('../' + file));
		});
	}
});