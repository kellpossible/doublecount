//! A double entry accounting system/library.
//!
//! # Optional Features
//!
//! The doublecount package has the following optional cargo features:
//!
//! + `serde-support`
//!   + Disabled by default
//!   + Enables support for serialization/de-serialization via `serde`
//!
//! # Usage
//!
//! ```
//! use doublecount::{
//!     AccountStatus, EditAccountStatus, Account, Program, Action,
//!     ProgramState, Transaction, TransactionElement, BalanceAssertion,
//!     ActionTypeValue,
//! };
//! use commodity::{CommodityType, Commodity};
//! use chrono::NaiveDate;
//! use std::rc::Rc;
//! use std::str::FromStr;
//!
//! // create a commodity from a currency's iso4317 alphanumeric code
//! let aud = Rc::from(CommodityType::from_str("AUD", "Australian Dollar").unwrap());
//!
//! // Create a couple of accounts
//! let account1 = Rc::from(Account::new_with_id(Some("Account 1"), aud.id, None));
//! let account2 = Rc::from(Account::new_with_id(Some("Account 2"), aud.id, None));
//!
//! // create a new program state, with accounts starting Closed
//! let mut program_state = ProgramState::new(
//!     &vec![account1.clone(), account2.clone()],
//!     AccountStatus::Closed
//! );
//!
//! // open account1
//! let open_account1 = EditAccountStatus::new(
//!     account1.id,
//!     AccountStatus::Open,
//!     NaiveDate::from_str("2020-01-01").unwrap(),
//! );
//!
//! // open account2
//! let open_account2 = EditAccountStatus::new(
//!     account2.id,
//!     AccountStatus::Open,
//!     NaiveDate::from_str("2020-01-01").unwrap(),
//! );
//!
//! // create a transaction to transfer some commodity
//! // from account1 to account2.
//! let transaction1 = Transaction::new(
//!     Some(String::from("Transaction 1")),
//!     NaiveDate::from_str("2020-01-02").unwrap(),
//!     vec![
//!         TransactionElement::new(
//!             account1.id,
//!             Some(Commodity::from_str("-2.52 AUD").unwrap()),
//!             None,
//!         ),
//!         TransactionElement::new(
//!             account2.id,
//!             Some(Commodity::from_str("2.52 AUD").unwrap()),
//!             None,
//!         ),
//!     ],
//! );
//!
//! // create a balance assertion (that will cause the program to return an error
//! // if it fails), to check that the balance of account1 matches the expected
//! // value of -1.52 AUD at the start of the date of 2020-01-03
//! let balance_assertion1 = BalanceAssertion::new(
//!     account1.id,
//!     NaiveDate::from_str("2020-01-03").unwrap(),
//!     Commodity::from_str("-2.52 AUD").unwrap()
//! );
//!
//! // create another transaction to transfer commodity from
//! // account2 to account1, using the simpler syntax.
//! let transaction2 =  Transaction::new_simple(
//!    Some("Transaction 2"),
//!    NaiveDate::from_str("2020-01-03").unwrap(),
//!    account2.id,
//!    account1.id,
//!    Commodity::from_str("1.0 AUD").unwrap(),
//!    None,
//! );
//!
//! let balance_assertion2 = BalanceAssertion::new(
//!     account1.id,
//!     NaiveDate::from_str("2020-01-04").unwrap(),
//!     Commodity::from_str("-1.52 AUD").unwrap()
//! );
//!
//! let balance_assertion3 = BalanceAssertion::new(
//!     account2.id,
//!     NaiveDate::from_str("2020-01-04").unwrap(),
//!     Commodity::from_str("1.52 AUD").unwrap()
//! );
//!
//! let actions: Vec<Rc<ActionTypeValue>> = vec![
//!     Rc::new(open_account1.into()),
//!     Rc::new(open_account2.into()),
//!     Rc::new(transaction1.into()),
//!     Rc::new(balance_assertion1.into()),
//!     Rc::new(transaction2.into()),
//!     Rc::new(balance_assertion2.into()),
//!     Rc::new(balance_assertion3.into()),
//! ];
//!
//! // create a program from the actions
//! let program = Program::new(actions);
//!
//! // run the program
//! program_state.execute_program(&program).unwrap();
//! ```

extern crate arrayvec;
extern crate chrono;
extern crate commodity;
extern crate nanoid;
extern crate rust_decimal;
extern crate thiserror;

#[cfg(feature = "serde-support")]
extern crate serde;

#[cfg(test)]
#[cfg(feature = "serde-support")]
extern crate serde_json;

mod account;
mod actions;
mod error;
mod program;

pub use account::*;
pub use actions::*;
pub use error::AccountingError;
pub use program::*;

#[cfg(doctest)]
#[macro_use]
extern crate doc_comment;

#[cfg(doctest)]
doctest!("../README.md");

#[cfg(test)]
mod tests {
    use super::{
        sum_account_states, Account, AccountState, AccountStatus, BalanceAssertion,
        EditAccountStatus, Program, ProgramState, Transaction, TransactionElement,
    };
    use crate::ActionTypeValue;
    use chrono::NaiveDate;
    use commodity::{Commodity, CommodityType, CommodityTypeID};
    use std::rc::Rc;
    use std::str::FromStr;

    #[test]
    fn execute_program() {
        let aud = Rc::from(CommodityType::new(
            CommodityTypeID::from_str("AUD").unwrap(),
            None,
        ));
        let account1 = Rc::from(Account::new_with_id(Some("Account 1"), aud.id, None));
        let account2 = Rc::from(Account::new_with_id(Some("Account 2"), aud.id, None));

        let accounts = vec![account1.clone(), account2.clone()];

        let mut program_state = ProgramState::new(&accounts, AccountStatus::Closed);

        let open_account1 = EditAccountStatus::new(
            account1.id,
            AccountStatus::Open,
            NaiveDate::from_str("2020-01-01").unwrap(),
        );

        let open_account2 = EditAccountStatus::new(
            account2.id,
            AccountStatus::Open,
            NaiveDate::from_str("2020-01-01").unwrap(),
        );

        let transaction1 = Transaction::new(
            Some(String::from("Transaction 1")),
            NaiveDate::from_str("2020-01-02").unwrap(),
            vec![
                TransactionElement::new(
                    account1.id,
                    Some(Commodity::from_str("-2.52 AUD").unwrap()),
                    None,
                ),
                TransactionElement::new(
                    account2.id,
                    Some(Commodity::from_str("2.52 AUD").unwrap()),
                    None,
                ),
            ],
        );

        let transaction2 = Transaction::new(
            Some(String::from("Transaction 2")),
            NaiveDate::from_str("2020-01-02").unwrap(),
            vec![
                TransactionElement::new(
                    account1.id,
                    Some(Commodity::from_str("-1.0 AUD").unwrap()),
                    None,
                ),
                TransactionElement::new(account2.id, None, None),
            ],
        );

        let balance_assertion = BalanceAssertion::new(
            account1.id,
            NaiveDate::from_str("2020-01-03").unwrap(),
            Commodity::from_str("-3.52 AUD").unwrap(),
        );

        let actions: Vec<Rc<ActionTypeValue>> = vec![
            Rc::new(open_account1.into()),
            Rc::new(open_account2.into()),
            Rc::new(transaction1.into()),
            Rc::new(transaction2.into()),
            Rc::new(balance_assertion.into()),
        ];

        let program = Program::new(actions);

        let account1_state_before: AccountState = program_state
            .get_account_state(&account1.id)
            .unwrap()
            .clone();

        assert_eq!(AccountStatus::Closed, account1_state_before.status);

        program_state.execute_program(&program).unwrap();

        let account1_state_after: AccountState = program_state
            .get_account_state(&account1.id)
            .unwrap()
            .clone();

        assert_eq!(AccountStatus::Open, account1_state_after.status);
        assert_eq!(
            Commodity::from_str("-3.52 AUD").unwrap(),
            account1_state_after.amount
        );

        assert_eq!(
            Commodity::from_str("0.0 AUD").unwrap(),
            sum_account_states(
                &program_state.account_states,
                CommodityTypeID::from_str("AUD").unwrap(),
                None
            )
            .unwrap()
        );
    }
}
