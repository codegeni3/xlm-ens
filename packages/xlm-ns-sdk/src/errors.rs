use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractErrorCode {
    NameNotFound = 1,
    NotOwner = 2,
    Expired = 3,
    InvalidLabel = 4,
    Other = 99,
}

#[derive(Debug)]
pub enum SdkError {
    InvalidRequest(String),
    Transport(String),
    ContractError(ContractErrorCode),
    /// The network passphrase returned by the RPC server does not
    /// match the passphrase configured in the SDK client.
    NetworkPassphraseMismatch {
        configured: String,
        rpc_reported: String,
    },
    /// The network passphrase embedded in a transaction does not
    /// match the passphrase configured in the SDK client.
    TransactionPassphraseMismatch {
        configured: String,
        in_transaction: String,
    },
}

impl fmt::Display for SdkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) => write!(f, "invalid request: {message}"),
            Self::Transport(message) => write!(f, "transport error: {message}"),
            Self::ContractError(code) => write!(f, "contract error: {code:?}"),
            Self::NetworkPassphraseMismatch {
                configured,
                rpc_reported,
            } => write!(
                f,
                "network passphrase mismatch: configured={configured:?}, rpc_reported={rpc_reported:?}"
            ),
            Self::TransactionPassphraseMismatch {
                configured,
                in_transaction,
            } => write!(
                f,
                "transaction passphrase mismatch: configured={configured:?}, in_transaction={in_transaction:?}"
            ),
        }
    }
}

impl std::error::Error for SdkError {}

pub fn decode_error(code: u32) -> ContractErrorCode {
    match code {
        1 => ContractErrorCode::NameNotFound,
        2 => ContractErrorCode::NotOwner,
        3 => ContractErrorCode::Expired,
        4 => ContractErrorCode::InvalidLabel,
        _ => ContractErrorCode::Other,
    }
}

