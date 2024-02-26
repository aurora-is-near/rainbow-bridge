#!/bin/bash
# This test launch all commands, submit a invalid block and challenge

set -exuo pipefail

CI_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/tmp/ganache.out 2>&1 && pwd )"
ROOT_DIR=$CI_DIR/../..

cd $ROOT_DIR/cli
yarn
node index.js clean
if [ -n "${LOCAL_CORE_SRC+x}" ]; then
  node index.js prepare --core-src "$LOCAL_CORE_SRC"
else
  node index.js prepare
fi
node index.js start near-node --archival true
# First start pm2 daemon
yarn run pm2 ping
node index.js start ganache
# Wait for the local node to start
while ! curl localhost:3030; do
  sleep 1
done

while ! curl localhost:9545; do
  sleep 1
done

node index.js init-near-contracts
(cd $ROOT_DIR/contracts/eth/nearbridge
yarn
yarn build)
(cd $ROOT_DIR/contracts/eth/nearprover
yarn
yarn build)
node index.js init-eth-ed25519
# Use short lockup time for tests
node index.js init-eth-client --eth-client-lock-eth-amount 1000000000000000000 --eth-client-lock-duration 30 --eth-client-replace-duration 60
node index.js init-eth-prover
node index.js init-eth-erc20
node index.js init-eth-locker
node index.js init-near-token-factory

sleep 5
yarn run pm2 list
node index.js start near2eth-relay --near2eth-relay-min-delay 1 --near2eth-relay-max-delay 30 --near2eth-relay-after-submit-delay-ms 45000 --near2eth-relay-block-select-duration 0 --eth-master-sk 0x2bdd21761a483f71054e14f5b827213567971c676928d9a1808cbfa4b7501201
sleep 5
yarn run pm2 list
node index.js start eth2near-relay
sleep 5
yarn run pm2 list
node index.js start bridge-watchdog --watchdog-delay 10
sleep 5
yarn run pm2 list

sleep 30
node index.js stop near2eth-relay
node index.js DANGER submit_invalid_near_block --near2eth-relay-block-select-duration 0
sleep 30
node index.js start near2eth-relay --near2eth-relay-min-delay 1 --near2eth-relay-max-delay 30 --near2eth-relay-after-submit-delay-ms 45000 --near2eth-relay-block-select-duration 0

node index.js TESTING transfer-eth-erc20-to-near --amount 1000 \
--eth-sender-sk 0x2bdd21761a483f71054e14f5b827213567971c676928d9a1808cbfa4b7501200 \
--near-receiver-account eth_on_near_prover.node0 --near-master-account eth_on_near_prover.node0 \
2>&1 | tee -a /tmp/eth2neartransfer.out
grep "Balance of eth_on_near_prover.node0 after the transfer is 1000" /tmp/eth2neartransfer.out
node index.js TESTING transfer-eth-erc20-from-near --amount 1 --near-sender-account eth_on_near_prover.node0 \
--near-sender-sk ed25519:3D4YudUQRE39Lc4JHghuB5WM8kbgDDa34mnrEP5DdTApVH81af7e2dWgNPEaiQfdJnZq1CNPp5im4Rg5b733oiMP \
--eth-receiver-address 0xEC8bE1A5630364292E56D01129E8ee8A9578d7D8 \
2>&1 | tee -a /tmp/near2ethtransfer.out
grep "after the transfer: 1" /tmp/near2ethtransfer.out
