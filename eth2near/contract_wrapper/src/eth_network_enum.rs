use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub enum EthNetwork {
    Mainnet,
    Kiln,
    Ropsten,
    Goerli,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IncorrectEthNetwork;

impl Display for IncorrectEthNetwork {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Unknown Eth network. Possible Eth network: 'Mainnet', 'Kiln', 'Goerli', 'Ropsten'"
        )
    }
}

impl Error for IncorrectEthNetwork {}

impl Display for EthNetwork {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            EthNetwork::Mainnet => write!(f, "mainnet"),
            EthNetwork::Kiln => write!(f, "kiln"),
            EthNetwork::Goerli => write!(f, "goerli"),
            EthNetwork::Ropsten => write!(f, "ropsten")
        }
    }
}

impl EthNetwork {
    pub fn as_str(&self) -> &str {
        match self {
            EthNetwork::Mainnet => "mainnet",
            EthNetwork::Kiln => "kiln",
            EthNetwork::Goerli => "goerli",
            EthNetwork::Ropsten => "ropsten"
        }
    }
}

impl FromStr for EthNetwork {
    type Err = IncorrectEthNetwork;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Mainnet" | "mainnet" => Ok(EthNetwork::Mainnet),
            "Kiln" | "kiln" => Ok(EthNetwork::Kiln),
            "Goerli" | "goerli" => Ok(EthNetwork::Goerli),
            "Ropsten" | "ropsten" => Ok(EthNetwork::Ropsten),
            _ => Err(IncorrectEthNetwork),
        }
    }
}
