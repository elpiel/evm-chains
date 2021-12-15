//! Crate containing the list of Ethereum Virtual Machine compatible chains.
//!
//! The crate loads the available chains list from the
//! [`ethereum-lists/chains`][ethereum-list-chains] as a `git` submodule.
//!
//! [ethereum-list-chains]: https://github.com/ethereum-lists/chains
use std::{collections::HashMap, fmt::Debug, fs::File, io::BufReader};

pub use error::Error;

use serde::{Deserialize, Serialize};

use once_cell::sync::Lazy;
static CHAINS: Lazy<HashMap<u64, Chain>> = Lazy::new(|| {
    let mut chains = HashMap::new();

    let chain_files = std::fs::read_dir("ethereum-list/chains/_data/chains/")
        .expect("Directory should be readable");

    for entry_result in chain_files {
        let dir_entry =
            entry_result.expect("Failed to read directory entry from chains data directory");

        let file_type = dir_entry
            .file_type()
            .expect("Failed to get the type of file entry in the chains data directory");

        let file_name = dir_entry
            .file_name()
            .into_string()
            .expect("Chain file name should contain valid Unicode string");

        // handle only files
        if !file_type.is_file() {
            continue;
        }
        // Strip the prefix `eip155-` & suffix `.json` of the file name
        let chain_id = file_name
            .strip_prefix("eip155-")
            .and_then(|file_name| file_name.strip_suffix(".json"))
            .expect("Chain file name was in incorrect form, expected: eip-155-CHAIN_ID.json")
            .parse::<u64>()
            .expect("Chain id in file name should be a valid `u64`");

        let chain = Chain::from_file(chain_id).unwrap_or_else(|err| {
            panic!(
                "Failed to read/deserialize chain file {}: {}",
                &file_name, err
            )
        });
        // if there is a chain with this ID already - panic!
        if chains.insert(chain_id, chain).is_some() {
            panic!("Duplicate Chain id ({})", chain_id)
        }
    }

    chains
});

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chain {
    /// E.g. "Ethereum Mainnet"
    pub name: String,
    /// E.g. "ETH"
    pub chain: String,
    /// E.g. "mainnet"
    pub network: String,
    /// the icon file from `ethereum-list/chains`
    pub icon: Option<String>,
    /// Urls for Remote procedure code (RPC) endpoints
    pub rpc: Vec<String>,
    /// Urls of faucets for the given Chain
    pub faucets: Vec<String>,
    pub native_currency: NativeCurrency,
    #[serde(rename = "infoURL")]
    pub info_url: String,
    pub short_name: String,
    pub chain_id: u64,
    pub network_id: u64,
    pub slip44: Option<u64>,
    pub ens: Option<Ens>,
    #[serde(default)]
    pub explorers: Vec<Explorer>,
}

impl Chain {
    pub fn from_file(chain_id: u64) -> Result<Self, Error> {
        let file_path = format!("ethereum-list/chains/_data/chains/eip155-{}.json", chain_id);

        let file = File::open(file_path).map_err(error::open_file)?;
        let reader = BufReader::new(file);

        serde_json::from_reader(reader).map_err(error::deserialize)
    }

    pub fn get(chain_id: u64) -> Option<Self> {
        CHAINS.get(&chain_id).cloned()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCurrency {
    pub name: String,
    pub symbol: String,
    pub decimals: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ens {
    /// `0x` prefixed and checksummed address
    pub registry: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Explorer {
    pub name: String,
    pub url: String,
    pub standard: String,
}

pub mod error {
    use std::{error::Error as StdError, fmt, fmt::Debug, io};

    use thiserror::Error;

    pub(crate) type BoxError = Box<dyn StdError + Send + Sync>;

    #[derive(Debug, Error)]
    #[error("{inner}")]
    pub struct Error {
        inner: Box<Inner>,
    }

    #[derive(Debug)]
    pub(crate) struct Inner {
        kind: Kind,
        source: Option<BoxError>,
    }

    impl fmt::Display for Inner {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match &self.source {
                Some(source) => write!(f, "{}: {}", self.kind, source),
                None => write!(f, "{}", self.kind),
            }
        }
    }

    impl fmt::Display for Kind {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Kind::Json => f.write_str("Deserializing json"),
                Kind::File => f.write_str("Reading file"),
            }
        }
    }
    impl Error {
        pub fn new<E: Into<BoxError>>(kind: Kind, source: Option<E>) -> Self {
            Self {
                inner: Box::new(Inner {
                    kind,
                    source: source.map(Into::into),
                }),
            }
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub enum Kind {
        Json,
        File,
    }

    pub(crate) fn open_file(error: io::Error) -> Error {
        Error::new(Kind::File, Some(error))
    }

    pub(crate) fn deserialize(error: serde_json::Error) -> Error {
        Error::new(Kind::Json, Some(error))
    }
}

#[cfg(test)]
mod tests {
    use super::{Chain, CHAINS};

    static ETHEREUM_FILE: &str = include_str!("../ethereum-list/chains/_data/chains/eip155-1.json");

    #[test]
    fn deserialize_file() {
        let _ethereum_chain =
            serde_json::from_str::<Chain>(ETHEREUM_FILE).expect("Should deserialize chain file");
    }

    #[test]
    fn chain_from_file() {
        let _ethereum_chain = Chain::from_file(1).expect("Should read and deserialize Chain");
    }

    #[test]
    fn get_chain() {
        // 1 - Eth mainnet
        // 56 - Binance mainnet
        // 137 - Polygon mainnet
        let get_chain_ids: [u64; 3] = [1, 56, 137];

        // first make sure that static is loading all files correctly
        for chain_id in get_chain_ids {
            let _chain = CHAINS
                .get(&chain_id)
                .unwrap_or_else(|| panic!("Chain({}) should exist", chain_id));
        }

        for chain_id in get_chain_ids {
            let _chain =
                Chain::get(chain_id).unwrap_or_else(|| panic!("Chain({}) should exist", chain_id));
        }
    }
}
