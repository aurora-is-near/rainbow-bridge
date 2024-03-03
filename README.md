<div align="center">

  <h1><code>Rainbow Bridge CLI</code></h1>

  <p>
    <strong>OPS tool to Rainbow Bridge, an Ethereum to Near trustless, fully decentralized, bidirectional bridge</strong>
  </p>

  <p>
    <a href="https://buildkite.com/nearprotocol/rainbow-bridge"><img alt="Build status" src="https://badge.buildkite.com/93478642b0ddf8e3548c16d2e60c4adbca4fd853520b6a5bca.svg"/></a>
    <a href="https://github.com/near/rainbow-bridge"><img alt="Bridge Version" src="https://img.shields.io/github/package-json/v/aurora-is-near/rainbow-bridge"></a>
    <a href="https://t.me/rainbowbridgesupport"><img alt="Telegram Chat" src="https://img.shields.io/badge/telegram-chat-blue?logo=telegram"></a>
  </p>
</div>

## Table of Contents

- [Pre-requisites](#pre-requisites)
- [Usage](#usage)
- [Security](#security)
- [Gas costs](#gas-costs)
- [Using Bridge on Testnet](#using-bridge-on-testnet)
- [Deploying and Using Locally](#deploying-and-using-locally)
- [Contract Development Workflow](#contract-development-workflow)

## Pre-requisites

The current version of CLI is all-in-one package -- it is used both for production and testing. As a result, even if you
need CLI only for the token transfer you need to install all testing dependencies. This will be changed in the future.

- Install golang, [see](https://golang.org/dl/).
- Make sure you are using Node with version >=12 and <=13. We recommend using [nvm](https://github.com/nvm-sh/nvm) for installing node and npm, if you already don't have one. This constraint will be removed soon;
- yarn
- docker, for deterministic compile rust contracts
- bash, for preparation steps (needs to be re-implemented in JS)

### If you want to test with a local near node:

- You would also need to install resources needed to compile nearcore (in the future this will only be required for the testing CLI):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup default stable
rustup target add wasm32-unknown-unknown
```

- Then install dependencies needed for the compilation of nearcore, [see](https://docs.near.org/docs/develop/node/validator/running-a-node#compiling-and-running-a-node-without-docker).
- python3 pip , for nearup

#### Install nearup

```bash
pip3 install nearup

```

## Usage

Clone this repo, `yarn install`, then to learn the commands that you can use with the rainbow bridge run:

```
    cli/index.js --help
```

Parameters of each command can be specified through environment variables, command line arguments, entries in the `~/.rainbow/config.json` config file, or the default value will be used -- in that priority.
If argument is not provided and there is no default value the program will not execute.

If script successfully executes a command then each parameter provided through the command line argument will be
written into the config file. Additionally, if scripts generates new parameters (e.g. it deploys a contract to Ethereum
and obtains its address) will also be written into the config file. Arguments should not be specified multiple times.

Note, you can use environment variables to pass sensitive data which will not lead to it being written into the config file.

## Security

* [Guide](near2eth/watchdog/README.md) on how to quickly configure the watchdog for Mainnet

Bridge is secure as long as majority (1/2) of Ethereum mining power is honest and supermajority (2/3) of NEAR stake is honest.
There are no additional security requirements, except that Ethereum should be able to accept 1 transaction within 4 hour period even in the worst congestion scenario.

## Gas costs

NEAR fees are negligible, both for bridge maintenance and for token transfer.
Ethereum fees are the following:

- To transfer ERC20 token from ETH to NEAR: Approx 43,989 gas to set allowance and approx 37,407 gas to lock it;
- To transfer ERC20 token back from NEAR to ETH: Approx 240,531 gas to unlock the token;
- To submit a NEAR block header: approx 697,140 gas;
- To challenge a NEAR block header: approx 700k gas.

As of 2020-07-14 (gas price is 40 gwei) the cost of running bridge on NEAR mainnet and Ethereum mainnet is approx 42 USD/day. The cost of ETH->NEAR transfer of ERC20 token is 1 USD. The cost of NEAR->ETH transfer of ERC20 token is 2 USD.

## Using Bridge on Testnet

### PoA vs PoW Ethereum networks

Rainbow bridge can be deployed either on PoW or PoA networks. However, the main use case of the bridge is Ethereum Mainnet, which makes its design very PoW-centric and it is only trustless and decentralized for PoW networks. Unfortunately, the only popular PoW testnet is Ropsten, which frequently undergoes huge reorgs of more than [16k blocks](https://github.com/near/rainbow-bridge-cli/issues/329), because people test 51% attacks on it. 16k reorgs can wipe out entire contracts and revert days of computations. Overall, Ropsten has the following unfortunate specifics that do not exist with Ethereum Mainnet:

- Extremely long re-orgs;
- Gas price volatility -- Ropsten blocks might have orders of magnitude different median gas price;
- Slow block production -- sometimes Ropsten blocks are produced once per several minutes;
- [Infura is unreliable on Ropsten](https://github.com/near/rainbow-bridge-cli/issues/330)

Therefore we advise users to not use Ropsten for bridge testing. Instead, we recommend using one of Ethereum's PoA testnet. Unfortunately, PoA networks have a different header format and are also centralized by nature. Therefore when deploying bridge on PoA network please use `--near-client-trusted-signer` parameter. This will force `EthOnNearClient` to not validate Ethereum headers (since PoA headers are not valid PoW headers) and accept them only from the provided authority.

The documentation below assumes Rinkeby testnet.

### Using existing bridge on Rinkeby

This section explains how to use existing bridge with mock ERC20 token that was already deployed. You would need to have some amount of this token on Rinkeby, so reach out to max@near.org if you want to give it a try.

We assume you have two accounts:

- One NEAR account on NEAR testnet with at least 1 NEAR token. We denote it as `<near_token_holder_account>` and its secret key as `<near_token_holder_sk>`;
- One Ethereum account on Rinkeby testnet with at least 1 ETH and 100 ERC20 tokens (this example uses ERC20 deployed to `0x8151a8F90267bFf183E06921841C5dE774499388` as an example. If you want some of these ERC20 tokens please contact max@near.org). We denote it as `<eth_token_holder_address>` and its private key as `<eth_token_holder_sk>`;

Make sure you have rainbow cli installed:

```bash
yarn install
```

If you have already used the bridge on this machine run a cleanup:

```bash
cli/index.js clean
```

If you're using rainbow-bridge-cli 1.x, create `~/.rainbow/config.json` file with the following content:

```json
{
  "nearNetworkId": "testnet",
  "nearNodeUrl": "https://rpc.testnet.near.org/",
  "ethNodeUrl": "https://rinkeby.infura.io/v3/<project_id>",
  "nearMasterAccount": "<near_token_holder_account>",
  "nearMasterSk": "<near_token_holder_sk>",
  "nearClientAccount": "ethonnearclient10",
  "nearProverAccount": "ethonnearprover10",
  "nearClientTrustedSigner": "eth2nearrelay10.testnet",
  "ethMasterSk": "<eth_token_holder_sk>",
  "ethEd25519Address": "0x9003342d15B21b4C42e1702447fE2f39FfAF55C2",
  "ethClientAddress": "0xF721c979db97413AA9D0F91ad531FaBF769bb09C",
  "ethProverAddress": "0xc5D62d66B8650E6242D9936c7e50E959BA0F9E37",
  "ethErc20Address": "0x8151a8F90267bFf183E06921841C5dE774499388",
  "ethLockerAddress": "0x5f7Cc23F90b5264a083dcB3b171c7111Dc32dD00",
  "nearFunTokenAccount": "mintablefuntoken11"
}
```

If you are using rainbow-bridge-cli 2.x, create `~/.rainbow/config.json` file with the following content:

```json
{
  "nearNetworkId": "testnet",
  "nearNodeUrl": "https://rpc.testnet.near.org/",
  "ethNodeUrl": "https://rinkeby.infura.io/v3/<project_id>",
  "nearMasterAccount": "<near_token_holder_account>",
  "nearMasterSk": "<near_token_holder_sk>",
  "nearClientAccount": "ethonnearclient10",
  "nearProverAccount": "ethonnearprover10",
  "nearClientTrustedSigner": "eth2nearrelay10.testnet",
  "ethMasterSk": "<eth_token_holder_sk>",
  "ethEd25519Address": "0x9003342d15B21b4C42e1702447fE2f39FfAF55C2",
  "ethClientAddress": "0xF721c979db97413AA9D0F91ad531FaBF769bb09C",
  "ethProverAddress": "0xc5D62d66B8650E6242D9936c7e50E959BA0F9E37",
  "nearTokenFactoryAccount": "ntf4.bridge2.testnet",
  "ethErc20Address": "0x21e7381368baa3f3e9640fe19780c4271ad96f37",
  "ethLockerAddress": "0x7f66c116a4f51e43e7c1c33d3714a4acfa9c40fb",
  "nearErc20Account": "21e7381368baa3f3e9640fe19780c4271ad96f37.ntf4.bridge2.testnet"
}
```

You can get infura project id, by registering at [infura.io](http://infura.io/).

To transfer ERC20 from ETH to NEAR run:

```bash
cli/index.js TESTING transfer-eth-erc20-to-near --amount 10 --eth-sender-sk <eth_token_holder_address> --near-receiver-account <near_token_holder_account>
```

(If the command interrupts in the middle re-run it and it will resume the transfer. PoA RPC sometimes has issues)
Wait for the transfer to finish. You should see:

```
Transferred
Balance of <near_token_holder_account> after the transfer is 10
```

To transfer ERC20 back from NEAR to ETH run:

```bash
cli/index.js TESTING transfer-eth-erc20-from-near --amount 1 --near-sender-account <near_token_holder_account> --near-sender-sk <near_token_holder_sk> --eth-receiver-address <eth_token_holder_address>
```

You should see:

```
ERC20 balance of <eth_token_holder_address> after the transfer: 91
```

Congratulations, you have achieved a roundtrip of ERC20 token through the bridge!

<!---
### Deploying new bridge

If you used bridge before from your machine, then clean up the setup. We recommend using cloud instance for deploying and running the bridge. Go to a cloud instance and install dependencies from [Pre-requisites](#pre-requisites).
Then run:
```bash
cli/index.js clean
cli/index.js prepare
```

Then initialize `EthOnNearClient` and `EthOnNearProver`:
```bash
cli/index.js init-near-contracts --near-network-id testnet --near-node-url <testnet_nodes_url> --eth-node-url https://ropsten.infura.io/v3/<infura_project_id> --near-master-account <near_master_account> --near-master-sk <near_master_sk> --near-client-account ethonnearclient01 --near-client-init-balance 2000000000000000000000000000 --near-prover-account ethonnearprover01
```
* Make sure `ethonnearclient01` and `ethonnearprover01` do not exist yet. You can check it by going to https://explorer.testnet.near.org/accounts/ethonnearclient01 and https://explorer.testnet.near.org/accounts/ethonnearprover01 . If they exist, pick different names;
* You can get `<infura_project_id>` by creating a free [infura](http://infura.io/) account. If you are working in NEAR organization please ask max@near.org;
* For `<testnet_nodes_url>` you can use `http://rpc.testnet.near.org/`. If you are working in NEAR organization please ask max@near.org;

Then start `eth2near-relay`:
```bash
node index.js start eth2near-relay --near-master-account <eth2nearrelay_account> --near-master-sk <eth2nearrelay_sk>
```

Now initialize `NearOnEthClient` and `NearOnEthProver`:
```bash
node index.js init-eth-ed25519 --eth-master-sk <eth_master_sk>
node index.js init-eth-client --eth-client-lock-eth-amount 100000000000000000 --eth-client-lock-duration 600
node index.js init-eth-prover
```
This will set the bond to 0.1 ETH and challenge period to 10 minutes. **Do not use these settings on Mainnet!** Mainnet should be using 20ETH bond and 4 hour challenge period.

Then start the `near2eth-relay` and watchdog:
```bash
node index.js start near2eth-relay --eth-master-sk <near2ethrelay_sk>
node index.js start bride-watchdog --eth-master-sk <watchdog_sk>
```
-->

## Deploying and Using Locally

To locally test the bridge run:

```bash
cli/index.js clean
cli/index.js prepare
cli/index.js start near-node
cli/index.js start ganache
```

### Initializing the contracts

First let's initialize the contracts that bridge needs to function:

```bash
cli/index.js init-near-contracts
cli/index.js init-eth-ed25519
cli/index.js init-eth-client --eth-client-lock-eth-amount 1000 --eth-client-lock-duration 10
cli/index.js init-eth-prover
```

Now, let's set up token on Ethereum blockchain that we can transfer to NEAR blockchain (this can be your own token).

```bash
cli/index.js init-eth-erc20
cli/index.js init-eth-locker
```

Now, let's initialize token factory on NEAR blockchain.

```bash
cli/index.js init-near-token-factory
```

### Starting the services

Now start the services that will relay the information between the chains:

```bash
cli/index.js start eth2near-relay
cli/index.js start near2eth-relay --eth-master-sk 0x2bdd21761a483f71054e14f5b827213567971c676928d9a1808cbfa4b7501201
cli/index.js start bridge-watchdog --eth-master-sk 0x2bdd21761a483f71054e14f5b827213567971c676928d9a1808cbfa4b7501202
```

Note, you can observe the logs of the relays by running:

```bash
pm2 logs
```

### Transferring tokens

Let's check the balance of bridged tokens from ETH to NEAR before starting the transfer. To this end let's use `node0` account, which is automatically created and funded on startup when localnet is started.

```bash
cli/index.js TESTING get-bridge-on-near-balance --near-receiver-account node0
```

Then transfer some tokens with:

```bash
cli/index.js TESTING transfer-eth-erc20-to-near --amount 1000 --eth-sender-sk 0x2bdd21761a483f71054e14f5b827213567971c676928d9a1808cbfa4b7501200 --near-receiver-account node0 --near-master-account neartokenfactory
```

Now you check the balance of `node0` again. You should notice the balance was changed.

Note, when we deployed ERC20 to the Ethereum blockchain we have minted a large number of tokens to the default master
key of Ganache, so we have transferred ERC20 tokens from it to `node0` account.
Notice that we are using `neartokenfactory.node0` account here to pay for the NEAR gas fees, any account for which we know a secret key would've worked too.
You must observe blocks being submitted.

You can also manually check the ERC20 balance of the receiver before and after receiving the transfer back from the NEAR side

```bash
cli/index.js TESTING get-erc20-balance 0xEC8bE1A5630364292E56D01129E8ee8A9578d7D8
```

Now let's try to transfer one token back to Ethereum

```bash
cli/index.js TESTING transfer-eth-erc20-from-near --amount 1 --near-sender-account node0 --near-sender-sk ed25519:3D4YudUQRE39Lc4JHghuB5WM8kbgDDa34mnrEP5DdTApVH81af7e2dWgNPEaiQfdJnZq1CNPp5im4Rg5b733oiMP --eth-receiver-address 0xEC8bE1A5630364292E56D01129E8ee8A9578d7D8
```

You should observe the change of the ERC20 balance as reported by the CLI.

### Stopping the services

To stop relay services and node clients execute the following command:

```bash
cli/index.js stop all
```

Or you can stop them one by one using these commands:

```bash
cli/index.js stop near-node
cli/index.js stop ganache
cli/index.js stop eth2near-relay
cli/index.js stop near2eth-relay
cli/index.js stop bridge-watchdog
```

## Contract Development Workflow

Above steps are ways to run a local bridge and development workflows you need if make any changes to rainbow-bridge-cli. If you want to update any of solidity or rust contracts, they're not in this repo now and workflow is as following.

- Install dependencies:

```bash
cli/index.js clean
cli/index.js prepare
```

- Start local NEAR network and Ganache

```bash
cli/index.js near-node
cli/index.js ganache
```

- If you want to modify solidity contracts, go to `node_modules/rainbow-bridge-sol`, make changes there and run `./build_all.sh` to recompile solidity contracts.
- If you want to modify rust contracts, go to `node_modules/ranbow-bridge-rs`, make changes there and run `./build_all.sh` to recompile rust contracts.
- If you want to modify rainbow bridge lib, go to `node_modules/rainbow-bridge-lib` and make changes there
- Follow instructions above to init eth contracts and near contracts, start services and start testing with bridge
- For changes to Solidity contract, Rust contract, and rainbow-bridge-lib, please submit PRs to: https://github.com/near/rainbow-bridge-sol , https://github.com/near/rainbow-bridge-rs , and https://github.com/near/rainbow-bridge-lib respectively.
- After PR merged in contract repos and rainbow-bridge-lib repo, we will periodically publish them as new version of npm packages. And rainbow-bridge-cli will adopt new version of them.

<!---
The following is outdated.
# Docker:

## Currently we have the following docker options:

1. Rainbow Docker image containing rainbow ready for running
   - run the rainbow docker image with a custom command
2. A development docker compose setup (docker-compose-dev.yml)
   - ganache
   - local near node
   - eth2near-relay
3. A production docker compose setup (docker-compose-prod.yml)
   - eth2near-relay

## Running the docker setup:

1. One options is to adapt the current config.json specified in the root folder of the project and build a new image.
2. Specifying the configuration flags through environment variables.

We recommend a usage of both, encouraging using the config.json for common configurations, while passing the secrets through environment variables.

Examples:

```
# Creating a docker image
docker build .

# Running the development env with config setup
docker-compose up

# Running the development env with ENV overrides
docker-compose -f docker-compose-dev.yml up -e MASTER_SK=<key> -e ...

# Running the production env just use:
docker-compose -f docker-compose-prod.yml instead
```
-->
