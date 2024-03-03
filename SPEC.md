# Rainbow Bridge Specification

## Overview

The Rainbow bridge is a composition of software applications allowing smart contracts in different blockchains to establish trustless communication between them. It is accomplished by having a mutual "smart contract"-based light clients in both blockchains; and cryptographic proofs of the including events (execution results) of smart contracts in blockchain blocks.

## Architecture

```
    +------------------------+     +------------------------+
    | Ethereum Blockchain    |     |        NEAR Blockchain |
    |                        |     |                        |
    |        +------------+  | (1) |  +-----------+         |
    |        |            |  |=======>|           |         |
    |    (A) | NearBridge |  |     |  | EthBridge | (B)     |
    |        |            |<=======|  |           |         |
    |        +------------+  | (2) |  +-----------+         |
    |              / \       |     |       / \              |
    |              |3|       |     |       |4|              |
    |        +------------+  |     |  +-----------+         |
    |    (C) | NearProver |  |     |  | EthProver | (D)     |
    |        +------------+  |     |  +-----------+         |
    |              / \       |     |       / \              |
    |              |5|       |     |       |6|              |
    |        +------------+  |     |  +-----------+         |
    |      +------------+ |  |     |  | +-----------+       |
    |    +------------+ |-+  |     |  +-| +-----------+     |
    |    |   . . .    |-+    |     |    +-|   . . .   |     |
    |    +------------+      |     |      +-----------+     |
    |                        |     |                        |
    +------------------------+     +------------------------+
```

Software:
- **A.** *NearBridge* – smart contract of Near light client hosted in Ethereum network. It receives Near block headers, verifies and stores block hashes only.
- **B.** *EthBridge* – smart contract of Ethereum light client hosted in Near network. It receives Ethereum block headers, verifies ethash and longest chain rule and stores block hashes only.
- **C.** *NearProver* - smart contract in Ethereum network performing verification of Near transaction result was included into Near block. Uses Merkle trees and hash preimages for verification.
- **D.** *EthProver* - smart contract in Near network performing verification of Ethereum event was included into Ethereum block. Uses Merkle trees and hash preimages for verification.

Relations:
1. Non-trusted and non-authorized Ethereum relayer software (aka *EthRelayer*) could forward Ethereum block headers into *EthBridge* smart contract hosted in Near blockchain.
2. Non-trusted and non-authorized Near relayer software (aka *NearRelayer*) could forward Near block headers into *NearBridge* smart contract hosted in Ethereum network.
3. *NearProver* verifies Near transaction result was included into Near bloc. And then checks if this block image exists in *NearBridge*.
4. *EthProver* verifies Ethereum event/log was included into Ethereum transaction receipt which was included into Ethereum block. And then checks if this block image exists in *EthBridge*.
