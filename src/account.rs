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
/// with a type of [CommodityType](commodity::CommodityType).
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Account {
    /// A unique identifier for this `Account`, currently generated using [nanoid](nanoid).
    pub id: AccountID,

    /// The name of this `Account`
    pub name: Option<String>,

    /// The id of the type of commodity to be stored in this account
    pub commodity_type_id: CommodityTypeID,

    /// The category that this account part of
    pub category: Option<AccountCategory>,
}

impl Account {
    /// Create a new account with an automatically generated id (using
    /// [nanoid](nanoid)) and add it to this program state (and create
    /// its associated [AccountState](AccountState)).
    pub fn new_with_id<S: Into<String>>(
        name: Option<S>,
        commodity_type_id: CommodityTypeID,
        category: Option<AccountCategory>,
    ) -> Account {
        let id_string: String = nanoid!(ACCOUNT_ID_LENGTH);
        Self::new(
            ArrayString::from(id_string.as_ref()).unwrap_or_else(|_| {
                panic!(
                    "generated id string {0} should fit within ACCOUNT_ID_LENGTH: {1}",
                    id_string, ACCOUNT_ID_LENGTH
                )
            }),
            name,
            commodity_type_id,
            category,
        )
    }

    /// Create a new account and add it to this program state (and create its associated
    /// [AccountState](AccountState)).
    pub fn new<S: Into<String>>(
        id: AccountID,
        name: Option<S>,
        commodity_type_id: CommodityTypeID,
        category: Option<AccountCategory>,
    ) -> Account {
        Account {
            id,
            name: name.map(|s| s.into()),
            commodity_type_id,
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

#[cfg(feature = "serde-support")]
#[cfg(test)]
mod serde_tests {
    use super::Account;
    use super::AccountID;
    use commodity::CommodityTypeID;
    use std::str::FromStr;

    #[test]
    fn account_serde() {
        use serde_json;

        let json = r#"{
  "id": "ABCDEFGHIJKLMNOPQRST",
  "name": "Test Account",
  "commodity_type_id": "USD",
  "category": "Expense"
}"#;

        let account: Account = serde_json::from_str(json).unwrap();

        let reference_account = Account::new(
            AccountID::from("ABCDEFGHIJKLMNOPQRST").unwrap(),
            Some("TestAccount"),
            CommodityTypeID::from_str("AUD").unwrap(),
            Some("Expense".to_string()),
        );

        assert_eq!(reference_account, account);
        insta::assert_json_snapshot!(account);
    }
}
