//! The `solc --standard-json` output error.

pub mod source_location;

use std::str::FromStr;

use serde::Deserialize;
use serde::Serialize;

use self::source_location::SourceLocation;

/// The `solc --standard-json` output error.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    /// The component type.
    pub component: String,
    /// The error code.
    pub error_code: Option<String>,
    /// The formatted error message.
    pub formatted_message: String,
    /// The non-formatted error message.
    pub message: String,
    /// The error severity.
    pub severity: String,
    /// The error location data.
    pub source_location: Option<SourceLocation>,
    /// The error type.
    pub r#type: String,
}

impl Error {
    /// Returns the `ecrecover` function usage warning.
    pub fn message_ecrecover(src: Option<&str>) -> Self {
        let message = r#"
Warning: It looks like you are using 'ecrecover' to validate a signature of a user account.
Polkadot comes with native account abstraction support, therefore it is highly recommended NOT
to rely on the fact that the account has an ECDSA private key attached to it since accounts might
implement other signature schemes.
"#
        .to_owned();

        Self {
            component: "general".to_owned(),
            error_code: None,
            formatted_message: message.clone(),
            message,
            severity: "warning".to_owned(),
            source_location: src.map(SourceLocation::from_str).and_then(Result::ok),
            r#type: "Warning".to_owned(),
        }
    }

    /// Returns the `<address payable>`'s `send` and `transfer` methods usage error.
    pub fn message_send_and_transfer(src: Option<&str>) -> Self {
        let message = r#"
Warning: It looks like you are using '<address payable>.send/transfer(<X>)'.
Using '<address payable>.send/transfer(<X>)' is deprecated and strongly discouraged!
The resolc compiler uses a heuristic to detect '<address payable>.send/transfer(<X>)' calls,
which disables call re-entrancy and supplies all remaining gas instead of the 2300 gas stipend.
However, detection is not guaranteed. You are advised to carefully test this, employ
re-entrancy guards or use the withdrawal pattern instead!
Learn more on https://docs.soliditylang.org/en/latest/security-considerations.html#reentrancy
and https://docs.soliditylang.org/en/latest/common-patterns.html#withdrawal-from-contracts
"#
        .to_owned();

        Self {
            component: "general".to_owned(),
            error_code: None,
            formatted_message: message.clone(),
            message,
            severity: "warning".to_owned(),
            source_location: src.map(SourceLocation::from_str).and_then(Result::ok),
            r#type: "Warning".to_owned(),
        }
    }

    /// Returns the `extcodesize` instruction usage warning.
    pub fn message_extcodesize(src: Option<&str>) -> Self {
        let message = r#"
Warning: Your code or one of its dependencies uses the 'extcodesize' instruction, which is
usually needed in the following cases:
  1. To detect whether an address belongs to a smart contract.
  2. To detect whether the deploy code execution has finished.
Polkadot comes with native account abstraction support (so smart contracts are just accounts
coverned by code), and you should avoid differentiating between contracts and non-contract
addresses.
"#
        .to_owned();

        Self {
            component: "general".to_owned(),
            error_code: None,
            formatted_message: message.clone(),
            message,
            severity: "warning".to_owned(),
            source_location: src.map(SourceLocation::from_str).and_then(Result::ok),
            r#type: "Warning".to_owned(),
        }
    }

    /// Returns the `origin` instruction usage warning.
    pub fn message_tx_origin(src: Option<&str>) -> Self {
        let message = r#"
Warning: You are checking for 'tx.origin' in your code, which might lead to unexpected behavior.
Polkadot comes with native account abstraction support, and therefore the initiator of a
transaction might be different from the contract calling your code. It is highly recommended NOT
to rely on tx.origin, but use msg.sender instead.
"#
        .to_owned();

        Self {
            component: "general".to_owned(),
            error_code: None,
            formatted_message: message.clone(),
            message,
            severity: "warning".to_owned(),
            source_location: src.map(SourceLocation::from_str).and_then(Result::ok),
            r#type: "Warning".to_owned(),
        }
    }

    /// Appends the contract path to the message..
    pub fn push_contract_path(&mut self, path: &str) {
        self.formatted_message
            .push_str(format!("\n--> {path}\n").as_str());
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.formatted_message)
    }
}
