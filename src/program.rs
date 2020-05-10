use super::{
    Account, AccountID, AccountState, AccountStatus, AccountingError, Action, ActionOrder,
    FailedBalanceAssertion,
};
use commodity::exchange_rate::ExchangeRate;
use commodity::{Commodity, CommodityTypeID};
use std::collections::HashMap;
use std::rc::Rc;

use crate::{ActionTypeValue};
#[cfg(feature = "serde-support")]
use serde::{de, Deserialize, Deserializer};

/// A collection of [Action](Action)s to be executed in order to
/// mutate some [ProgramState](ProgramState).
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub actions: Vec<Rc<ActionTypeValue>>,
}

impl Program {
    /// Create a new [Program](Program).
    ///
    /// The provided `actions` will be sorted using [ActionOrder](ActionOrder).
    pub fn new(actions: Vec<Rc<ActionTypeValue>>) -> Program {
        let mut sorted_actions = actions;
        sorted_actions.sort_by_key(|a| ActionOrder(a.clone()));
        Program {
            actions: sorted_actions,
        }
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }
}

#[cfg(feature = "serde-support")]
struct ProgramVisitor;

#[cfg(feature = "serde-support")]
impl<'de> de::Visitor<'de> for ProgramVisitor {
    type Value = Program;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(format!("Program comprising of a vector of Actions",).as_ref())
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Program, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut actions: Vec<Rc<ActionTypeValue>> = match seq.size_hint() {
            Some(size_hint) => Vec::with_capacity(size_hint),
            None => Vec::new(),
        };

        while let Some(action) = seq.next_element::<ActionTypeValue>()? {
            actions.push(Rc::new(action));
        }

        Ok(Program::new(actions))
    }
}

#[cfg(feature = "serde-support")]
impl<'de> Deserialize<'de> for Program {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Program, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(ProgramVisitor)
    }
}

/// The state of a [Program](Program) being executed.
pub struct ProgramState {
    /// list of states associated with accounts (can only grow)
    pub account_states: HashMap<AccountID, AccountState>,

    /// list of failed assertions, and associated failed balance
    pub failed_balance_assertions: Vec<FailedBalanceAssertion>,

    /// the index of the currently executing action
    current_action_index: usize,
}

/// Sum the values in all the accounts into a single
/// [Commodity](Commodity), and use the supplied exchange rate if
/// required to convert a type of commodity in an account to the
/// [CommidityType](commodity::CommodityType) associated with the
/// id `sum_commodity_type_id`.
pub fn sum_account_states(
    account_states: &HashMap<AccountID, AccountState>,
    sum_commodity_type_id: CommodityTypeID,
    exchange_rate: Option<&ExchangeRate>,
) -> Result<Commodity, AccountingError> {
    let mut sum = Commodity::zero(sum_commodity_type_id);

    for (_, account_state) in account_states {
        let account_amount = if account_state.amount.type_id != sum_commodity_type_id {
            match exchange_rate {
                Some(rate) => rate.convert(account_state.amount, sum_commodity_type_id)?,
                None => {
                    return Err(AccountingError::NoExchangeRateSupplied(
                        account_state.amount,
                        sum_commodity_type_id,
                    ))
                }
            }
        } else {
            account_state.amount
        };

        sum = sum.add(&account_amount)?;
    }

    Ok(sum)
}

impl ProgramState {
    /// Create a new [ProgramState](ProgramState).
    pub fn new(accounts: &Vec<Rc<Account>>, account_status: AccountStatus) -> ProgramState {
        let mut account_states = HashMap::new();

        for account in accounts {
            account_states.insert(
                account.id,
                AccountState::new(
                    account.clone(),
                    Commodity::zero(account.commodity_type_id),
                    account_status,
                ),
            );
        }

        ProgramState {
            account_states,
            failed_balance_assertions: Vec::new(),
            current_action_index: 0,
        }
    }

    /// Execute a given [Program](Program) to mutate this state.
    pub fn execute_program(&mut self, program: &Program) -> Result<(), AccountingError> {
        for (index, action) in program.actions.iter().enumerate() {
            action.as_action().perform(self)?;
            self.current_action_index = index;
        }

        // TODO: change this to return a list of failed assertions in the error
        match self.failed_balance_assertions.get(0) {
            Some(failed_assertion) => {
                return Err(AccountingError::BalanceAssertionFailed(
                    failed_assertion.clone(),
                ));
            }
            None => {}
        };

        Ok(())
    }

    /// Get the reference to an [Account](Account) using it's [AccountID](AccountID).
    pub fn get_account(&self, account_id: &AccountID) -> Option<&Account> {
        self.get_account_state(account_id)
            .map(|state| state.account.as_ref())
    }

    /// Get a reference to the `AccountState` associated with a given `Account`.
    pub fn get_account_state(&self, account_id: &AccountID) -> Option<&AccountState> {
        self.account_states.get(account_id)
    }

    /// Get a mutable reference to the `AccountState` associated with the given `Account`.
    pub fn get_account_state_mut(&mut self, account_id: &AccountID) -> Option<&mut AccountState> {
        self.account_states.get_mut(account_id)
    }

    /// Record a failed [BalanceAssertion](super::BalanceAssertion)
    /// using a [FailedBalanceAssertion](FailedBalanceAssertion).
    pub fn record_failed_balance_assertion(
        &mut self,
        failed_balance_assertion: FailedBalanceAssertion,
    ) {
        self.failed_balance_assertions
            .push(failed_balance_assertion);
    }
}

#[cfg(test)]
mod tests {
    use super::Program;
    use crate::{Action, BalanceAssertion, TransactionElement, Transaction, EditAccountStatus, AccountStatus, Account, AccountID, ActionTypeValue};
    use std::{str::FromStr, rc::Rc};
    use chrono::NaiveDate;
    use commodity::{CommodityType, Commodity, CommodityTypeID};

    #[test]
    fn program_serde() {
        let json = r#"
[
    {
        "type": "EditAccountStatus",
        "account_id": "TestAccount1",
        "newstatus": "Open",
        "date": "2020-01-01"
    },
    {
        "type": "EditAccountStatus",
        "account_id": "TestAccount2",
        "newstatus": "Open",
        "date": "2020-01-01"
    },
    {
        "type": "Transaction",
        "description": "Test Transaction",
        "date": "2020-01-02",
        "elements": [
            {
                "account_id": "TestAccount1",
                "amount": {
                    "value": "-2.52",
                    "type_id": "AUD"
                }
            },
            {
                "account_id": "TestAccount2",
                "amount": {
                    "value": "2.52",
                    "type_id": "AUD"
                }
            }
        ]  
    },
    {
        "type": "BalanceAssertion",
        "account_id": "TestAccount1",
        "date": "2020-01-03",
        "expected_balance": {
            "value": "-3.52",
            "type_id": "AUD"
        }
    }
]"#;
        let program: Program = serde_json::from_str(json).unwrap();

        let aud = Rc::from(CommodityType::new(
            CommodityTypeID::from_str("AUD").unwrap(),
            None,
        ));
        
        let account1 = Rc::from(Account::new(AccountID::from("TestAccount1").unwrap(),Some("Test Account 1"), aud.id, None));
        let account2 = Rc::from(Account::new(AccountID::from("TestAccount2").unwrap(),Some("Test Account 2"), aud.id, None));

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

        let transaction = Transaction::new(
            Some(String::from("Test Transaction")),
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

        let balance_assertion = BalanceAssertion::new(
            account1.id,
            NaiveDate::from_str("2020-01-03").unwrap(),
            Commodity::from_str("-3.52 AUD").unwrap(),
        );

        let actions: Vec<Rc<ActionTypeValue>> = vec![
            Rc::new(open_account1.into()),
            Rc::new(open_account2.into()),
            Rc::new(transaction.into()),
            Rc::new(balance_assertion.into()),
        ];

        let reference_program = Program::new(actions);

        assert_eq!(reference_program, program);
    }
}
