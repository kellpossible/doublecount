use super::{
    Account, AccountID, AccountState, AccountStatus, AccountingError, Action, ActionOrder,
    FailedBalanceAssertion,
};
use commodity::exchange_rate::ExchangeRate;
use commodity::{Commodity, CurrencyCode};
use std::collections::HashMap;
use std::rc::Rc;

/// A collection of [Action](Action)s to be executed in order to
/// mutate some [ProgramState](ProgramState).
pub struct Program {
    actions: Vec<Rc<dyn Action>>,
}

impl Program {
    /// Create a new [Program](Program).
    ///
    /// The provided `actions` will be sorted using [ActionOrder](ActionOrder).
    pub fn new(actions: Vec<Rc<dyn Action>>) -> Program {
        let mut sorted_actions = actions;
        sorted_actions.sort_by_key(|a| ActionOrder(a.clone()));
        Program {
            actions: sorted_actions,
        }
    }
}

// #[cfg(feature = "serde-support")]
// impl<'de> Deserialize<'de> for Program {
//     fn deserialize<D>(deserializer: D) -> std::result::Result<Program, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         use serde::de::{self, Visitor};

//         struct ProgramVisitor;

//         impl<'de> Visitor<'de> for ProgramVisitor {
//             type Value = Program;

//             fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
//                 formatter.write_str(
//                     format!(
//                         "a string with a maximum of {} characters",
//                         commodity::CURRENCY_CODE_LENGTH
//                     )
//                     .as_ref(),
//                 )
//             }

//             fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
//             where
//                 E: de::Error,
//             {
//                 CurrencyCode::from_str(v).map_err(|e| {
//                     E::custom(format!(
//                         "there was an error ({}) parsing the currency code string",
//                         e
//                     ))
//                 })
//             }
//         }

//         deserializer.deserialize_str(CurrencyCodeVisitor)
//     }
// }

/// The state of a [Program](Program) being executed.
pub struct ProgramState {
    /// list of states associated with accounts (can only grow)
    pub account_states: HashMap<AccountID, AccountState>,

    /// list of failed assertions, and associated failed balance
    pub failed_balance_assertions: Vec<FailedBalanceAssertion>,

    /// the index of the currently executing action
    current_action_index: usize,
}

/// Sum the values in all the accounts into a single [Commodity](Commodity), and
/// use the supplied exchange rate if required to convert a currency in an account
/// to the `sum_currency`.
pub fn sum_account_states(
    account_states: &HashMap<AccountID, AccountState>,
    sum_currency: CurrencyCode,
    exchange_rate: Option<&ExchangeRate>,
) -> Result<Commodity, AccountingError> {
    let mut sum = Commodity::zero(sum_currency);

    for (_, account_state) in account_states {
        let account_amount = if account_state.amount.currency_code != sum_currency {
            match exchange_rate {
                Some(rate) => rate.convert(account_state.amount, sum_currency)?,
                None => {
                    return Err(AccountingError::NoExchangeRateSupplied(
                        account_state.amount,
                        sum_currency,
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
                    Commodity::zero(account.currency_code),
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
            action.perform(self)?;
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
