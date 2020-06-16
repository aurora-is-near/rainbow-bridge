
const { program } = require('commander');

const { CleanCommand } = require('./commands/clean');
const { PrepareCommand } = require('./commands/prepare');
const { StartEthRelayCommand } = require('./commands/start/eth-relay.js');
const { StartGanacheNodeCommand } = require('./commands/start/ganache.js');
const { StartLocalNearNodeCommand } = require('./commands/start/near.js');
const { StopLocalNearNodeCommand } = require('./commands/stop/near.js');
const { StopManagedProcessCommand } = require('./commands/stop/process.js');
const { TransferETHERC20ToNear } = require('./commands/transfer-eth-erc20-to-near');
const { InitETHLocker } = require('./commands/init-eth-locker');
const { InitETHERC20 } = require('./commands/init-eth-erc20');
const { InitNEARContracts } = require('./commands/init-near-contracts');
const { InitNEARFunToken } = require('./commands/init-near-fun-token');
const { ETHDump } = require('./commands/eth-dump');
const { RainbowConfig } = require('./lib/config');

RainbowConfig.declareOption(
    'near-network-id',
    'The identifier of the NEAR network that the given NEAR node is expected to represent.',
);
RainbowConfig.declareOption(
    'near-node-url',
    'The URL of the NEAR node.',
);
RainbowConfig.declareOption(
    'eth-node-url',
    'The URL of the Ethereum node.',
);
RainbowConfig.declareOption(
    'near-master-account',
    'The account of the master account on NEAR blockchain that can be used to deploy and initialize the test contracts.' +
    ' This account will also own the initial supply of the fungible tokens.',
);
RainbowConfig.declareOption(
    'near-master-sk',
    'The secret key of the master account on NEAR blockchain.',
);
RainbowConfig.declareOption(
    'eth-master-sk',
    'The secret key of the master account on Ethereum blockchain.',
);
RainbowConfig.declareOption(
    'eth2near-client-account',
    'The account of the Eth2NearClient contract that can be used to accept ETH headers.',
    'eth2nearclient',
);
RainbowConfig.declareOption(
    'eth2near-client-sk',
    'The secret key of the Eth2NearClient account. If not specified will use master SK.',
);
RainbowConfig.declareOption(
    'eth2near-client-contract-path',
    'The path to the Wasm file containing the Eth2NearClient contract.',
);
RainbowConfig.declareOption(
    'eth2near-client-init-balance',
    'The initial balance of Eth2NearClient contract in femtoNEAR.',
    '100000000000000000000000000',
);
RainbowConfig.declareOption(
    'eth2near-client-validate-ethash',
    'The initial balance of Eth2NearClient contract in femtoNEAR.',
    'true',
);
RainbowConfig.declareOption(
    'eth2near-prover-account',
    'The account of the Eth2NearProver contract that can be used to accept ETH headers.',
    'eth2nearprover',
);
RainbowConfig.declareOption(
    'eth2near-prover-sk',
    'The secret key of the Eth2NearProver account. If not specified will use master SK.',
);
RainbowConfig.declareOption(
    'eth2near-prover-contract-path',
    'The path to the Wasm file containing the Eth2NearProver contract.',
);
RainbowConfig.declareOption(
    'eth2near-prover-init-balance',
    'The initial balance of Eth2NearProver contract in femtoNEAR.',
    '100000000000000000000000000',
);
RainbowConfig.declareOption('daemon', 'Whether the process should be launched as a daemon.', 'true', true);
RainbowConfig.declareOption('bridge-src', 'Path to the rainbow-bridge source. It will be downloaded if not provided.');
RainbowConfig.declareOption('core-src', 'Path to the nearcore source. It will be downloaded if not provided.');
RainbowConfig.declareOption('nearup-src', 'Path to the nearup source. It will be downloaded if not provided.');

// User-specific arguments.
RainbowConfig.declareOption(
    'near-fun-token-account',
    'The account of the fungible token contract that will be used to mint tokens locked on Ethereum.',
    'nearfuntoken',
);
RainbowConfig.declareOption(
    'near-fun-token-sk',
    'The secret key of the fungible token account. If not specified will use master SK.',
);
RainbowConfig.declareOption(
    'near-fun-token-contract-path',
    'The path to the Wasm file containing the fungible contract. Note, this version of fungible contract should support minting.',
);
RainbowConfig.declareOption(
    'near-fun-token-init-balance',
    'The initial balance of fungible token contract in femtoNEAR.',
    '100000000000000000000000000',
);
RainbowConfig.declareOption(
    'eth-locker-address',
    'ETH address of the locker contract.',
);
RainbowConfig.declareOption(
    'eth-locker-abi-path',
    'Path to the .abi file definining Ethereum locker contract. This contract works in pair with mintable fungible token on NEAR blockchain.',
);
RainbowConfig.declareOption(
    'eth-locker-bin-path',
    'Path to the .bin file definining Ethereum locker contract. This contract works in pair with mintable fungible token on NEAR blockchain.',
);
RainbowConfig.declareOption(
    'eth-erc20-address',
    'ETH address of the ERC20 contract.',
);
RainbowConfig.declareOption(
    'eth-erc20-abi-path',
    'Path to the .abi file definining Ethereum ERC20 contract.',
);
RainbowConfig.declareOption(
    'eth-erc20-bin-path',
    'Path to the .bin file definining Ethereum ERC20 contract.',
);

program.version('0.1.0');

// General-purpose commands.
program.command('clean').action(CleanCommand.execute);

RainbowConfig.addOptions(
    program.command('prepare')
        .action(PrepareCommand.execute),
    [
        'bridge-src',
        'core-src',
        'nearup-src',
    ]);

// Maintainer commands.

const startCommand = program.command('start');

startCommand.command('near-node')
    .action(StartLocalNearNodeCommand.execute);

RainbowConfig.addOptions(
    startCommand.command('ganache')
        .action(StartGanacheNodeCommand.execute),
    ['daemon'],
);

RainbowConfig.addOptions(
    startCommand.command('eth-relay')
        .action(StartEthRelayCommand.execute),
    [
        'near-master-account',
        'near-master-sk',
        'eth2near-client-account',
        'near-network-id',
        'near-node-url',
        'daemon',
    ],
);

const stopCommand = program.command('stop');

stopCommand.command('near-node')
    .action(StopLocalNearNodeCommand.execute);

stopCommand.command('ganache')
    .action(StopManagedProcessCommand.execute);

stopCommand.command('eth-relay')
    .action(StopManagedProcessCommand.execute);

RainbowConfig.addOptions(
    program.command('init-near-contracts')
        .description('Deploys and initializes Eth2NearClient and Eth2NearProver contracts to NEAR blockchain.')
        .action(InitNEARContracts.execute),
    [
        'near-network-id',
        'near-node-url',
        'eth-node-url',
        'near-master-account',
        'near-master-sk',
        'eth2near-client-account',
        'eth2near-client-sk',
        'eth2near-client-contract-path',
        'eth2near-client-init-balance',
        'eth2near-client-validate-ethash',
        'eth2near-prover-account',
        'eth2near-prover-sk',
        'eth2near-prover-contract-path',
        'eth2near-prover-init-balance',
    ]);

// User commands.

RainbowConfig.addOptions(
    program.command('init-near-fun-token')
        .description('Deploys and initializes mintable fungible token to NEAR blockchain. Requires locker on Ethereum side.')
        .action(InitNEARFunToken.execute),
    [
        'near-fun-token-account',
        'near-fun-token-sk',
        'near-fun-token-contract-path',
        'near-fun-token-init-balance',
    ],
);

RainbowConfig.addOptions(
    program.command('init-eth-locker')
        .description('Deploys and initializes locker contract on Ethereum blockchain. Requires mintable fungible token on Near side.')
        .action(InitETHLocker.execute),
    [
        'eth-node-url',
        'eth-master-sk',
        'eth-locker-abi-path',
        'eth-locker-bin-path',
    ],
);

RainbowConfig.addOptions(
    program.command('init-eth-erc20')
        .description('Deploys and initializes ERC20 contract on Ethereum blockchain.')
        .action(InitETHERC20.execute),
    [
        'eth-node-url',
        'eth-master-sk',
        'eth-erc20-abi-path',
        'eth-erc20-bin-path',
    ],
);

RainbowConfig.addOptions(
    program.command('transfer-eth-erc20-to-near')
        .action(TransferETHERC20ToNear.execute)
        .option('--amount <amount>', 'Amount of ERC20 tokens to transfer')
        .option('--eth-sender-sk <eth_sender_sk>', 'The secret key of the Ethereum account that will be sending ERC20 token.')
        .option('--near-receiver-account <near_receiver_account>', 'The account on NEAR blockchain that will be receiving the minted token.'),
    [
        'eth-node-url',
        'eth-erc20-address',
        'eth-erc20-abi-path',
        'eth-locker-address',
        'eth-locker-abi-path',
        'near-node-url',
        'near-network-id',
        'near-fun-token-account',
        'eth2near-client-account',
        'near-master-account',
        'near-master-sk',
    ],
);

program.command('eth-dump <kind_of_data>')
    .option('--eth-node-url <eth_node_url>', 'ETH node API url')
    .option('--path <path>', 'Dir path to dump eth headers')
    .option('--start-block <start_block>', 'Start block number (inclusive), default to be 4.3K blocks away from start block')
    .option('--end-block <end_block>', 'End block number (inclusive), default to be latest block')
    .action(ETHDump.execute);

(async () => { await program.parseAsync(process.argv); })();
