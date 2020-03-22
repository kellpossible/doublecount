# Doublecount [![crates.io badge](https://img.shields.io/crates/v/doublecount.svg)](https://crates.io/crates/doublecount) [![license badge](https://img.shields.io/github/license/kellpossible/doublecount)](https://github.com/kellpossible/doublecount/blob/master/LICENSE.txt) [![docs.rs badge](https://docs.rs/doublecount/badge.svg)](https://docs.rs/doublecount/)

A double entry accounting system/library for Rust.

This project is very much inspired by [beancount](http://furius.ca/beancount/),
however it currently presents a much simpler model. It has been designed to
embed within other applications for the purpose of running accounting
calculations.

Commodities within the system are represented using the primitives provided by
the [commodity](https://crates.io/crates/commodity) library, which is in turn
backed by [rust_decimal](https://crates.io/crates/rust_decimal).

This library is under active development, however it should already be usable
for some simple purposes. There's likely to be some API changes in the future to
allow transactions/actions to be streamed into the system, and also to support
parallel computations of transactions to allow large programs to efficiently
executed on multi-core computers.

**[Changelog](./CHANGELOG.md)**

## Optional Features

The following features can be enabled to provide extra functionality:

+ `serde-support`
  + **Currently incomplete**
  + Enables support for serialization/de-serialization via `serde`

## Usage

```rust
use doublecount::{
    AccountStatus, EditAccountStatus, Account, Program, Action,
    ProgramState, Transaction, TransactionElement, BalanceAssertion,
};
use commodity::{CommodityType, Commodity};
use chrono::NaiveDate;
use std::rc::Rc;
use std::str::FromStr;

// create a currency from its iso4317 alphanumeric code
let aud = Rc::from(CommodityType::from_currency_alpha3("AUD").unwrap());

// Create a couple of accounts
let account1 = Rc::from(Account::new(Some("Account 1"), aud.id, None));
let account2 = Rc::from(Account::new(Some("Account 2"), aud.id, None));

// create a new program state, with accounts starting Closed
let mut program_state = ProgramState::new(
    &vec![account1.clone(), account2.clone()],
    AccountStatus::Closed
);

// open account1
let open_account1 = EditAccountStatus::new(
    account1.id,
    AccountStatus::Open,
    NaiveDate::from_str("2020-01-01").unwrap(),
);

// open account2
let open_account2 = EditAccountStatus::new(
    account2.id,
    AccountStatus::Open,
    NaiveDate::from_str("2020-01-01").unwrap(),
);

// create a transaction to transfer some commodity
// from account1 to account2.
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

// create a balance assertion (that will cause the program to return an error
// if it fails), to check that the balance of account1 matches the expected
// value of -1.52 AUD at the start of the date of 2020-01-03
let balance_assertion1 = BalanceAssertion::new(
    account1.id,
    NaiveDate::from_str("2020-01-03").unwrap(),
    Commodity::from_str("-2.52 AUD").unwrap()
);

// create another transaction to transfer commodity from
// account2 to account1, using the simpler syntax.
let transaction2 =  Transaction::new_simple(
   Some("Transaction 2"),
   NaiveDate::from_str("2020-01-03").unwrap(),
   account2.id,
   account1.id,
   Commodity::from_str("1.0 AUD").unwrap(),
   None,
);

let balance_assertion2 = BalanceAssertion::new(
    account1.id,
    NaiveDate::from_str("2020-01-04").unwrap(),
    Commodity::from_str("-1.52 AUD").unwrap()
);

let balance_assertion3 = BalanceAssertion::new(
    account2.id,
    NaiveDate::from_str("2020-01-04").unwrap(),
    Commodity::from_str("1.52 AUD").unwrap()
);

let actions: Vec<Rc<dyn Action>> = vec![
    Rc::from(open_account1),
    Rc::from(open_account2),
    Rc::from(transaction1),
    Rc::from(balance_assertion1),
    Rc::from(transaction2),
    Rc::from(balance_assertion2),
    Rc::from(balance_assertion3),
];

// create a program from the actions
let program = Program::new(actions);

// run the program
program_state.execute_program(&program).unwrap();
```
