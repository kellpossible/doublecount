use super::{Account, AccountID, AccountStatus, FailedBalanceAssertion, Transaction};
use commodity::exchange_rate::ExchangeRateError;
use commodity::{Commodity, CommodityError, CurrencyCode};
use std::rc::Rc;
use thiserror::Error;

/// An error associated with functionality in the [accounting](./index.html) module.
///
/// TODO: add context for the error for where it occurred within the [Program](super::Program)
#[derive(Error, Debug)]
pub enum AccountingError {
    #[error("error relating to currencies")]
    Currency(#[from] CommodityError),
    #[error("error relating to exchange rates")]
    ExchangeRate(#[from] ExchangeRateError),
    #[error("invalid account status ({:?}) for account {}", .status, .account.id)]
    InvalidAccountStatus {
        account: Rc<Account>,
        status: AccountStatus,
    },
    #[error("error parsing a date from string")]
    DateParseError(#[from] chrono::ParseError),
    #[error("invalid transaction {0:?} because {1}")]
    InvalidTransaction(Transaction, String),
    #[error("failed checksum, the sum of account values in the common currency ({0}) does not equal zero")]
    FailedCheckSum(Commodity),
    #[error("no exchange rate supplied, unable to convert commodity {0} to currency {1}")]
    NoExchangeRateSupplied(Commodity, CurrencyCode),
    #[error("the account state with the id {0} was requested but cannot be found")]
    MissingAccountState(AccountID),
    #[error("the balance assertion failed {0}")]
    BalanceAssertionFailed(FailedBalanceAssertion),
}
