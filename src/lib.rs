//! A double entry accounting system/library.
//!
//! # Optional Features
//!
//! The doublecount package has the following optional cargo features:
//!
//! + `serde-support`
//!   + **Currently incomplete**
//!   + Disabled by default
//!   + Enables support for serialization/de-serialization via `serde`
//!   + Enables support for json serialization/de-serialization via `serde_json`
//! 
//! # Usage
//! 
//! ```
//! use doublecount::{
//!     AccountStatus, EditAccountStatus, Account, Program, Action,
//!     ProgramState, Transaction, TransactionElement, BalanceAssertion, 
//! };
//! use commodity::{Currency, Commodity};
//! use chrono::NaiveDate;
//! use std::rc::Rc;
//! use std::str::FromStr;
//! 
//! // create a currency from its iso4317 alphanumeric code
//! let aud = Rc::from(Currency::from_alpha3("AUD").unwrap());
//! 
//! // Create a couple of accounts
//! let account1 = Rc::from(Account::new(Some("Account 1"), aud.clone(), None));
//! let account2 = Rc::from(Account::new(Some("Account 2"), aud.clone(), None));
//! 
//! // create a new program state, with accounts starting Closed
//! let mut program_state = ProgramState::new(
//!     &vec![account1.clone(), account2.clone()], 
//!     AccountStatus::Closed
//! );
//! 
//! // open account1
//! let open_account1 = EditAccountStatus::new(
//!     account1.clone(),
//!     AccountStatus::Open,
//!     NaiveDate::from_str("2020-01-01").unwrap(),
//! );
//! 
//! // open account2
//! let open_account2 = EditAccountStatus::new(
//!     account2.clone(),
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
//!             account1.clone(),
//!             Some(Commodity::from_str("-2.52 AUD").unwrap()),
//!             None,
//!         ),
//!         TransactionElement::new(
//!             account2.clone(),
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
//!     account1.clone(),
//!     NaiveDate::from_str("2020-01-03").unwrap(),
//!     Commodity::from_str("-2.52 AUD").unwrap()
//! );
//! 
//! // create another transaction to transfer commodity from
//! // account2 to account1, using the simpler syntax.
//! let transaction2 =  Transaction::new_simple(
//!    Some("Transaction 2"),
//!    NaiveDate::from_str("2020-01-03").unwrap(),
//!    account2.clone(),
//!    account1.clone(),
//!    Commodity::from_str("1.0 AUD").unwrap(),
//!    None,
//! );
//! 
//! let balance_assertion2 = BalanceAssertion::new(
//!     account1.clone(),
//!     NaiveDate::from_str("2020-01-04").unwrap(),
//!     Commodity::from_str("-1.52 AUD").unwrap()
//! );
//! 
//! let balance_assertion3 = BalanceAssertion::new(
//!     account2.clone(),
//!     NaiveDate::from_str("2020-01-04").unwrap(),
//!     Commodity::from_str("1.52 AUD").unwrap()
//! );
//! 
//! let actions: Vec<Rc<dyn Action>> = vec![
//!     Rc::from(open_account1),
//!     Rc::from(open_account2),
//!     Rc::from(transaction1),
//!     Rc::from(balance_assertion1),
//!     Rc::from(transaction2),
//!     Rc::from(balance_assertion2),
//!     Rc::from(balance_assertion3),
//! ];
//! 
//! // create a program from the actions
//! let program = Program::new(actions);
//! 
//! // run the program
//! program_state.execute_program(&program).unwrap();
//! ```

extern crate chrono;
extern crate commodity;
extern crate nanoid;
extern crate rust_decimal;
extern crate thiserror;

#[cfg(feature = "serde-support")]
extern crate serde;
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

// create a list of actions with associated dates
// a transaction is a type of action
// opening an account is another type of action
// the list is a program which will be executed, to compute
// the final resulting values. All should add up to zero.

#[cfg(test)]
mod tests {
    use super::{
        sum_account_states, Account, AccountState, AccountStatus, Action, EditAccountStatus,
        Program, ProgramState, Transaction, TransactionElement, BalanceAssertion,
    };
    use chrono::NaiveDate;
    use commodity::{Commodity, Currency, CurrencyCode};
    use std::rc::Rc;
    use std::str::FromStr;

    #[test]
    fn execute_program() {
        let aud = Rc::from(Currency::new(CurrencyCode::from_str("AUD").unwrap(), None));
        let account1 = Rc::from(Account::new(Some("Account 1"), aud.clone(), None));
        let account2 = Rc::from(Account::new(Some("Account 2"), aud.clone(), None));

        let accounts = vec![account1.clone(), account2.clone()];

        let mut program_state = ProgramState::new(&accounts, AccountStatus::Closed);

        let open_account1 = EditAccountStatus::new(
            account1.clone(),
            AccountStatus::Open,
            NaiveDate::from_str("2020-01-01").unwrap(),
        );

        let open_account2 = EditAccountStatus::new(
            account2.clone(),
            AccountStatus::Open,
            NaiveDate::from_str("2020-01-01").unwrap(),
        );

        let transaction1 = Transaction::new(
            Some(String::from("Transaction 1")),
            NaiveDate::from_str("2020-01-02").unwrap(),
            vec![
                TransactionElement::new(
                    account1.clone(),
                    Some(Commodity::from_str("-2.52 AUD").unwrap()),
                    None,
                ),
                TransactionElement::new(
                    account2.clone(),
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
                    account1.clone(),
                    Some(Commodity::from_str("-1.0 AUD").unwrap()),
                    None,
                ),
                TransactionElement::new(account2.clone(), None, None),
            ],
        );

        let balance_assertion = BalanceAssertion::new(
            account1.clone(),
            NaiveDate::from_str("2020-01-03").unwrap(),
            Commodity::from_str("-3.52 AUD").unwrap()
        );

        let actions: Vec<Rc<dyn Action>> = vec![
            Rc::from(open_account1),
            Rc::from(open_account2),
            Rc::from(transaction1),
            Rc::from(transaction2),
            Rc::from(balance_assertion),
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
                CurrencyCode::from_str("AUD").unwrap(),
                None
            )
            .unwrap()
        );
    }
}
