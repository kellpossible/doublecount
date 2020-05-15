use super::{
    Account, AccountID, AccountState, AccountStatus, AccountingError, ActionOrder,
    FailedBalanceAssertion,
};
use commodity::exchange_rate::ExchangeRate;
use commodity::{Commodity, CommodityTypeID};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;

use crate::{ActionType, ActionTypeFor, ActionTypeValue, ActionTypeValueEnum};
#[cfg(feature = "serde-support")]
use serde::{de, ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};

/// A collection of [Action](Action)s to be executed in order to
/// mutate some [ProgramState](ProgramState).
#[derive(Debug, Clone, PartialEq)]
pub struct Program<AT = ActionType, ATV = ActionTypeValue> {
    pub actions: Vec<Rc<ATV>>,
    action_type: PhantomData<AT>,
}

impl<AT, ATV> Program<AT, ATV>
where
    AT: Ord,
    ATV: ActionTypeValueEnum<AT> + ActionTypeFor<AT>,
{
    /// Create a new [Program](Program).
    ///
    /// The provided `actions` will be sorted using [ActionOrder](ActionOrder).
    pub fn new(actions: Vec<Rc<ATV>>) -> Program<AT, ATV> {
        let mut sorted_actions: Vec<Rc<ATV>> = actions;
        sorted_actions.sort_by_key(|a| ActionOrder::new(a.clone()));
        Program {
            actions: sorted_actions,
            action_type: PhantomData::default(),
        }
    }

    /// The number of actions in this program.
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// Returns true if there are no actions in this progam.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

#[cfg(feature = "serde-support")]
struct ProgramVisitor<AT, ATV> {
    action_type: PhantomData<AT>,
    action_type_value: PhantomData<ATV>,
}

#[cfg(feature = "serde-support")]
impl<AT, ATV> ProgramVisitor<AT, ATV> {
    pub fn new() -> Self {
        Self {
            action_type: PhantomData::default(),
            action_type_value: PhantomData::default(),
        }
    }
}

#[cfg(feature = "serde-support")]
impl<'de, AT, ATV> de::Visitor<'de> for ProgramVisitor<AT, ATV>
where
    AT: Ord,
    ATV: Deserialize<'de> + ActionTypeValueEnum<AT> + ActionTypeFor<AT>,
{
    type Value = Program<AT, ATV>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(format!("Program comprising of a vector of Actions",).as_ref())
    }

    fn visit_seq<S>(self, mut seq: S) -> Result<Program<AT, ATV>, S::Error>
    where
        S: de::SeqAccess<'de>,
    {
        let mut actions: Vec<Rc<ATV>> = match seq.size_hint() {
            Some(size_hint) => Vec::with_capacity(size_hint),
            None => Vec::new(),
        };

        while let Some(action) = seq.next_element::<ATV>()? {
            actions.push(Rc::new(action));
        }

        Ok(Program::new(actions))
    }
}

#[cfg(feature = "serde-support")]
impl<'de, AT, ATV> Deserialize<'de> for Program<AT, ATV>
where
    AT: Ord,
    ATV: Deserialize<'de> + ActionTypeValueEnum<AT> + ActionTypeFor<AT>,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Program<AT, ATV>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(ProgramVisitor::<AT, ATV>::new())
    }
}

#[cfg(feature = "serde-support")]
impl<AT, ATV> Serialize for Program<AT, ATV>
where
    ATV: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.actions.len()))?;
        for e in &self.actions {
            seq.serialize_element(&**e)?;
        }
        seq.end()
    }
}

/// The state of a [Program](Program) being executed.
pub struct ProgramState<AT = ActionType, ATV = ActionTypeValue> {
    /// list of states associated with accounts (can only grow)
    pub account_states: HashMap<AccountID, AccountState>,

    /// list of failed assertions, and associated failed balance
    pub failed_balance_assertions: Vec<FailedBalanceAssertion>,

    /// the index of the currently executing action
    current_action_index: usize,

    action_type: PhantomData<AT>,
    action_type_value: PhantomData<ATV>,
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

    for account_state in account_states.values() {
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

impl<AT, ATV> ProgramState<AT, ATV>
where
    ATV: ActionTypeValueEnum<AT>,
{
    /// Create a new [ProgramState](ProgramState).
    pub fn new(accounts: &[Rc<Account>], account_status: AccountStatus) -> ProgramState<AT, ATV> {
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
            action_type: PhantomData::default(),
            action_type_value: PhantomData::default(),
        }
    }

    /// Execute a given [Program](Program) to mutate this state.
    pub fn execute_program(&mut self, program: &Program<AT, ATV>) -> Result<(), AccountingError> {
        for (index, action) in program.actions.iter().enumerate() {
            action.as_action().perform(self)?;
            self.current_action_index = index;
        }

        // TODO: change this to return a list of failed assertions in the error
        if let Some(failed_assertion) = self.failed_balance_assertions.get(0) {
            return Err(AccountingError::BalanceAssertionFailed(
                failed_assertion.clone(),
            ));
        }

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

#[cfg(feature = "serde-support")]
#[cfg(test)]
mod tests {
    use super::Program;
    use crate::{
        Account, AccountID, AccountStatus, ActionTypeValue, BalanceAssertion, EditAccountStatus,
        Transaction, TransactionElement,
    };
    use chrono::NaiveDate;
    use commodity::{Commodity, CommodityType, CommodityTypeID};
    use std::{rc::Rc, str::FromStr};

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

        let account1 = Rc::from(Account::new(
            AccountID::from("TestAccount1").unwrap(),
            Some("Test Account 1"),
            aud.id,
            None,
        ));
        let account2 = Rc::from(Account::new(
            AccountID::from("TestAccount2").unwrap(),
            Some("Test Account 2"),
            aud.id,
            None,
        ));

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

        insta::assert_json_snapshot!(program);
    }
}
