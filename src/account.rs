use arrayvec::ArrayString;
use commodity::{Commodity, CommodityTypeID};
use nanoid::nanoid;
use rust_decimal::Decimal;
use std::rc::Rc;

#[cfg(feature = "serde-support")]
use serde::{Deserialize, Serialize};

/// The size in characters/bytes of the [Account](Account) id.
const ACCOUNT_ID_LENGTH: usize = 20;

/// The status of an [Account](Account) stored within an [AccountState](AccountState).
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AccountStatus {
    /// The account is open
    Open,
    /// The account is closed
    Closed,
}
/// The type to use for the id of [Account](Account)s.
pub type AccountID = ArrayString<[u8; ACCOUNT_ID_LENGTH]>;

/// A way to categorize [Account](Account)s.
pub type AccountCategory = String;

/// Details for an account, which holds a [Commodity](Commodity)
/// with a type of [Currency](commodity::Currency).
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Account {
    /// A unique identifier for this `Account`, currently generated using [nanoid](nanoid).
    pub id: AccountID,

    /// The name of this `Account`
    pub name: Option<String>,

    /// The type of currency to be stored in this account
    pub commodity_type_id: CommodityTypeID,

    /// The category that this account part of
    pub category: Option<AccountCategory>,
}

impl Account {
    /// Create a new account and add it to this program state (and create its associated
    /// [AccountState](AccountState)).
    pub fn new(
        name: Option<&str>,
        commodity_type_id: CommodityTypeID,
        category: Option<AccountCategory>,
    ) -> Account {
        let id_string: String = nanoid!(ACCOUNT_ID_LENGTH);
        Account {
            id: ArrayString::from(id_string.as_ref()).expect(
                format!(
                    "generated id string {0} should fit within ACCOUNT_ID_LENGTH: {1}",
                    id_string, ACCOUNT_ID_LENGTH
                )
                .as_ref(),
            ),
            name: name.map(|s| String::from(s)),
            commodity_type_id: commodity_type_id,
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
