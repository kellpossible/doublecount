use commodity::{Commodity, Currency};
use nanoid::nanoid;
use rust_decimal::Decimal;
use std::rc::Rc;

const ACCOUNT_ID_SIZE: usize = 20;

#[derive(Copy, Clone, Debug, PartialEq)]
/// The status of an [Account](Account) stored within an [AccountState](AccountState).
pub enum AccountStatus {
    /// The account is open
    Open,
    /// The account is closed
    Closed,
}

/// A way to categorize [Account](Account)s.
#[derive(Debug)]
pub struct AccountCategory {
    /// The name of the category
    pub name: String,
    /// The parent category (or `None` if this is a root category)
    pub parent: Option<Rc<AccountCategory>>,
}

/// The type to use for the id of [Account](Account)s.
pub type AccountID = String;

/// Details for an account, which holds a [Commodity](Commodity)
/// with a type of [Currency](Currency).
#[derive(Debug, Clone)]
pub struct Account {
    /// A unique identifier for this `Account`
    pub id: AccountID,

    /// The name of this `Account`
    pub name: Option<String>,

    /// The type of currency to be stored in this account
    pub currency: Rc<Currency>,

    /// The category that this account part of
    pub category: Option<Rc<AccountCategory>>,
}

impl Account {
    /// Create a new account and add it to this program state (and create its associated
    /// [AccountState](AccountState)).
    pub fn new(
        name: Option<&str>,
        currency: Rc<Currency>,
        category: Option<Rc<AccountCategory>>,
    ) -> Account {
        Account {
            id: nanoid!(ACCOUNT_ID_SIZE),
            name: name.map(|s| String::from(s)),
            currency,
            category,
        }
    }
}

impl PartialEq for Account {
    fn eq(&self, other: &Account) -> bool {
        self.id == other.id
    }
}

/// Mutable state associated with an [Account](Account).
#[derive(Debug, Clone, PartialEq)]
pub struct AccountState {
    /// The [Account](Account) associated with this state
    pub account: Rc<Account>,

    /// The amount of the commodity currently stored in this account
    pub amount: Commodity,

    /// The status of this account (open/closed/etc...)
    pub status: AccountStatus,
}

impl AccountState {
    /// Create a new [AccountState](AccountState).
    pub fn new(account: Rc<Account>, amount: Commodity, status: AccountStatus) -> AccountState {
        AccountState {
            account,
            amount,
            status,
        }
    }

    /// Open this account, set the `status` to [Open](AccountStatus::Open)
    pub fn open(&mut self) {
        self.status = AccountStatus::Open;
    }

    // Close this account, set the `status` to [Closed](AccountStatus::Closed)
    pub fn close(&mut self) {
        self.status = AccountStatus::Closed;
    }

    pub fn eq_approx(&self, other: &AccountState, epsilon: Decimal) -> bool {
        self.account == other.account
            && self.status == other.status
            && self.amount.eq_approx(other.amount, epsilon)
    }
}
