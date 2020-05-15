use super::{AccountID, AccountStatus, AccountingError, ProgramState};
use chrono::NaiveDate;
use commodity::exchange_rate::ExchangeRate;
use commodity::Commodity;
use rust_decimal::{prelude::Zero, Decimal};
use std::fmt;
use std::rc::Rc;
use std::{marker::PhantomData, slice};

#[cfg(feature = "serde-support")]
use serde::{Deserialize, Serialize};

/// A representation of what type of [Action](Action) is being performed.
#[derive(PartialEq, Eq, Debug, PartialOrd, Ord, Hash, Clone)]
pub enum ActionType {
    /// An [Action](Action) to edit the status of an [Account](crate::Account).
    /// Represented by the [EditAccountStatus](EditAccountStatus) struct.
    ///
    /// This action has the highest priority when being sorted, because
    /// other actions on the same day may depend on this already having
    /// been executed.
    EditAccountStatus,
    /// An [Action](Action) to assert the current balance of an account while
    /// a [Program](super::Program) is being executed. Represented by a
    /// [BalanceAssertion](BalanceAssertion) struct.
    BalanceAssertion,
    /// A [Action](Action) to perform a transaction between [Account](crate::Account)s.
    /// Represented by the [Transaction](Transaction) struct.
    Transaction,
}

impl ActionTypeFor<ActionType> for ActionTypeValue {
    fn action_type(&self) -> ActionType {
        match self {
            ActionTypeValue::EditAccountStatus(_) => ActionType::EditAccountStatus,
            ActionTypeValue::BalanceAssertion(_) => ActionType::BalanceAssertion,
            ActionTypeValue::Transaction(_) => ActionType::Transaction,
        }
    }
}

impl ActionType {
    /// Return an iterator over all available [ActionType](ActionType) variants.
    pub fn iterator() -> slice::Iter<'static, ActionType> {
        static ACTION_TYPES: [ActionType; 3] = [
            ActionType::EditAccountStatus,
            ActionType::BalanceAssertion,
            ActionType::Transaction,
        ];
        ACTION_TYPES.iter()
    }
}

/// A trait which represents an enum/sized data structure which is
/// capable of storing every possible concrete implementation of
/// [Action](Action) for your [Program](crate::Program).
///
/// If you have some custom actions, you need to implement this trait
/// yourself and use it to store your actions that you provide to
/// [Program](crate::Program).
pub trait ActionTypeValueEnum<AT> {
    fn as_action(&self) -> &dyn Action<AT, Self>;
}

/// An enum to store every possible concrete implementation of
/// [Action](Action) in a `Sized` element.
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-support", serde(tag = "type"))]
#[derive(Debug, Clone, PartialEq)]
pub enum ActionTypeValue {
    EditAccountStatus(EditAccountStatus),
    BalanceAssertion(BalanceAssertion),
    Transaction(Transaction),
}

impl<AT> ActionTypeValueEnum<AT> for ActionTypeValue {
    fn as_action(&self) -> &dyn Action<AT, ActionTypeValue> {
        match self {
            ActionTypeValue::EditAccountStatus(action) => action,
            ActionTypeValue::BalanceAssertion(action) => action,
            ActionTypeValue::Transaction(action) => action,
        }
    }
}

impl From<EditAccountStatus> for ActionTypeValue {
    fn from(action: EditAccountStatus) -> Self {
        ActionTypeValue::EditAccountStatus(action)
    }
}

impl From<BalanceAssertion> for ActionTypeValue {
    fn from(action: BalanceAssertion) -> Self {
        ActionTypeValue::BalanceAssertion(action)
    }
}

impl From<Transaction> for ActionTypeValue {
    fn from(action: Transaction) -> Self {
        ActionTypeValue::Transaction(action)
    }
}

/// Obtain the concrete action type for an action.
pub trait ActionTypeFor<AT> {
    /// What type of action is being performed.
    fn action_type(&self) -> AT;
}

/// Represents an action which can modify [ProgramState](ProgramState).
pub trait Action<AT, ATV>: fmt::Display + fmt::Debug {
    /// The date/time (in the account history) that the action was performed.
    fn date(&self) -> NaiveDate;

    /// Perform the action to mutate the [ProgramState](ProgramState).
    fn perform(&self, program_state: &mut ProgramState<AT, ATV>) -> Result<(), AccountingError>;
}

/// A way to sort [Action](Action)s by their date, and then by the
/// priority of their [ActionType](ActionType).
///
/// # Example
/// ```
/// use doublecount::{ActionTypeValue, ActionOrder};
/// use std::rc::Rc;
///
/// let mut actions: Vec<Rc<ActionTypeValue>> = Vec::new();
///
/// // let's pretend we created and added
/// // some actions to the actions vector
///
/// // sort the actions using this order
/// actions.sort_by_key(|a| ActionOrder::new(a.clone()));
/// ```
pub struct ActionOrder<AT, ATV> {
    action_value: Rc<ATV>,
    action_type: PhantomData<AT>,
}

impl<AT, ATV> ActionOrder<AT, ATV> {
    pub fn new(action_value: Rc<ATV>) -> Self {
        Self {
            action_value,
            action_type: PhantomData::default(),
        }
    }
}

impl<AT, ATV> PartialEq for ActionOrder<AT, ATV>
where
    AT: PartialEq,
    ATV: ActionTypeValueEnum<AT> + ActionTypeFor<AT>,
{
    fn eq(&self, other: &ActionOrder<AT, ATV>) -> bool {
        let self_action = self.action_value.as_action();
        let other_action = other.action_value.as_action();
        self.action_value.action_type() == other.action_value.action_type()
            && self_action.date() == other_action.date()
    }
}

impl<AT, ATV> Eq for ActionOrder<AT, ATV>
where
    ATV: ActionTypeValueEnum<AT> + ActionTypeFor<AT>,
    AT: PartialEq,
{
}

impl<AT, ATV> PartialOrd for ActionOrder<AT, ATV>
where
    AT: Ord,
    ATV: ActionTypeValueEnum<AT> + ActionTypeFor<AT>,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let self_action = self.action_value.as_action();
        let other_action = other.action_value.as_action();
        self_action
            .date()
            .partial_cmp(&other_action.date())
            .map(|date_order| {
                date_order.then(
                    self.action_value
                        .action_type()
                        .cmp(&other.action_value.action_type()),
                )
            })
    }
}

impl<AT, ATV> Ord for ActionOrder<AT, ATV>
where
    AT: Ord,
    ATV: ActionTypeValueEnum<AT> + ActionTypeFor<AT>,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_action = self.action_value.as_action();
        let other_action = other.action_value.as_action();
        self_action.date().cmp(&other_action.date()).then(
            self.action_value
                .action_type()
                .cmp(&other.action_value.action_type()),
        )
    }
}

/// A movement of [Commodity](Commodity) between two or more accounts
/// on a given `date`. Implements [Action](Action) so it can be
/// applied to change [AccountState](super::AccountState)s.
///
/// The sum of the [Commodity](Commodity) `amount`s contained within a
/// transaction's [TransactionElement](TransactionElement)s needs to
/// be equal to zero, or one of the elements needs to have a `None`
/// value `amount`.
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct Transaction {
    /// Description of this transaction.
    pub description: Option<String>,
    /// The date that the transaction occurred.
    pub date: NaiveDate,
    /// Elements which compose this transaction.
    ///
    /// See [Transaction](Transaction) for more information about the
    /// constraints which apply to this field.
    pub elements: Vec<TransactionElement>,
}

impl Transaction {
    /// Create a new [Transaction](Transaction).
    pub fn new<S: Into<String>>(
        description: Option<S>,
        date: NaiveDate,
        elements: Vec<TransactionElement>,
    ) -> Transaction {
        Transaction {
            description: description.map(|s| s.into()),
            date,
            elements,
        }
    }

    /// Create a new simple [Transaction](Transaction), containing
    /// only two elements, transfering an `amount` from `from_account`
    /// to `to_account` on the given `date`, with the given
    /// `exchange_rate` (required if the currencies of the accounts
    /// are different).
    ///
    /// # Example
    /// ```
    /// # use doublecount::Transaction;
    /// # use std::rc::Rc;
    /// use doublecount::Account;
    /// use commodity::{CommodityType, Commodity};
    /// use chrono::Local;
    /// use std::str::FromStr;
    ///
    /// let aud = Rc::from(CommodityType::from_currency_alpha3("AUD").unwrap());
    ///
    /// let account1 = Rc::from(Account::new_with_id(Some("Account 1"), aud.id, None));
    /// let account2 = Rc::from(Account::new_with_id(Some("Account 2"), aud.id, None));
    ///
    /// let transaction = Transaction::new_simple(
    ///    Some("balancing"),
    ///    Local::today().naive_local(),
    ///    account1.id,
    ///    account2.id,
    ///    Commodity::from_str("100.0 AUD").unwrap(),
    ///    None,
    /// );
    ///
    /// assert_eq!(2, transaction.elements.len());
    /// let element0 = transaction.elements.get(0).unwrap();
    /// let element1 = transaction.elements.get(1).unwrap();
    /// assert_eq!(Some(Commodity::from_str("-100.0 AUD").unwrap()), element0.amount);
    /// assert_eq!(account1.id, element0.account_id);
    /// assert_eq!(account2.id, element1.account_id);
    /// assert_eq!(None, element1.amount);
    /// ```
    pub fn new_simple<S: Into<String>>(
        description: Option<S>,
        date: NaiveDate,
        from_account: AccountID,
        to_account: AccountID,
        amount: Commodity,
        exchange_rate: Option<ExchangeRate>,
    ) -> Transaction {
        Transaction::new(
            description,
            date,
            vec![
                TransactionElement::new(from_account, Some(amount.neg()), exchange_rate.clone()),
                TransactionElement::new(to_account, None, exchange_rate),
            ],
        )
    }

    /// Get the [TransactionElement](TransactionElement) associated
    /// with the given [Account](crate::Account)'s id.
    pub fn get_element(&self, account_id: &AccountID) -> Option<&TransactionElement> {
        self.elements.iter().find(|e| &e.account_id == account_id)
    }
}

impl ActionTypeFor<ActionType> for Transaction {
    fn action_type(&self) -> ActionType {
        todo!()
    }
}

impl fmt::Display for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Transaction")
    }
}

impl<AT, ATV> Action<AT, ATV> for Transaction
where
    ATV: ActionTypeValueEnum<AT>,
{
    fn date(&self) -> NaiveDate {
        self.date
    }

    fn perform(&self, program_state: &mut ProgramState<AT, ATV>) -> Result<(), AccountingError> {
        // check that the transaction has at least 2 elements
        if self.elements.len() < 2 {
            return Err(AccountingError::InvalidTransaction(
                self.clone(),
                String::from("a transaction cannot have less than 2 elements"),
            ));
        }

        //TODO: add check to ensure that transaction doesn't have duplicate account references?

        // first process the elements to automatically calculate amounts

        let mut empty_amount_element: Option<usize> = None;
        for (i, element) in self.elements.iter().enumerate() {
            if element.amount.is_none() {
                if empty_amount_element.is_none() {
                    empty_amount_element = Some(i)
                } else {
                    return Err(AccountingError::InvalidTransaction(
                        self.clone(),
                        String::from("multiple elements with no amount specified"),
                    ));
                }
            }
        }

        let sum_commodity_type_id = match empty_amount_element {
            Some(empty_i) => {
                let empty_element = self.elements.get(empty_i).unwrap();

                match program_state.get_account(&empty_element.account_id) {
                    Some(account) => account.commodity_type_id,
                    None => {
                        return Err(AccountingError::MissingAccountState(
                            empty_element.account_id,
                        ))
                    }
                }
            }
            None => {
                let account_id = self
                    .elements
                    .get(0)
                    .expect("there should be at least 2 elements in the transaction")
                    .account_id;

                match program_state.get_account(&account_id) {
                    Some(account) => account.commodity_type_id,
                    None => return Err(AccountingError::MissingAccountState(account_id)),
                }
            }
        };

        let mut sum = Commodity::new(Decimal::zero(), sum_commodity_type_id);

        let mut modified_elements = self.elements.clone();

        // Calculate the sum of elements (not including the empty element if there is one)
        for (i, element) in self.elements.iter().enumerate() {
            if let Some(empty_i) = empty_amount_element {
                if i != empty_i {
                    //TODO: perform commodity type conversion here if required
                    sum = match sum.add(&element.amount.as_ref().unwrap()) {
                        Ok(value) => value,
                        Err(error) => return Err(AccountingError::Commodity(error)),
                    }
                }
            }
        }

        // Calculate the value to use for the empty element (negate the sum of the other elements)
        if let Some(empty_i) = empty_amount_element {
            let modified_emtpy_element: &mut TransactionElement =
                modified_elements.get_mut(empty_i).unwrap();
            let negated_sum = sum.neg();
            modified_emtpy_element.amount = Some(negated_sum);

            sum = match sum.add(&negated_sum) {
                Ok(value) => value,
                Err(error) => return Err(AccountingError::Commodity(error)),
            }
        }

        if sum.value != Decimal::zero() {
            return Err(AccountingError::InvalidTransaction(
                self.clone(),
                String::from("sum of transaction elements does not equal zero"),
            ));
        }

        for transaction in &modified_elements {
            let mut account_state = program_state
                .get_account_state_mut(&transaction.account_id)
                .unwrap_or_else(||
                    panic!(
                        "unable to find state for account with id: {} please ensure this account was added to the program state before execution.",
                        transaction.account_id
                    )
                );

            match account_state.status {
                AccountStatus::Closed => Err(AccountingError::InvalidAccountStatus {
                    account_id: transaction.account_id,
                    status: account_state.status,
                }),
                _ => Ok(()),
            }?;

            // TODO: perform the commodity type conversion using the exchange rate (if present)

            let transaction_amount = match &transaction.amount {
                Some(amount) => amount,
                None => {
                    return Err(AccountingError::InvalidTransaction(
                        self.clone(),
                        String::from(
                            "unable to calculate all required amounts for this transaction",
                        ),
                    ))
                }
            };

            account_state.amount = match account_state.amount.add(transaction_amount) {
                Ok(commodity) => commodity,
                Err(err) => {
                    return Err(AccountingError::Commodity(err));
                }
            }
        }

        Ok(())
    }
}

/// An element of a [Transaction](Transaction).
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct TransactionElement {
    /// The account to perform the transaction to
    pub account_id: AccountID,

    /// The amount of [Commodity](Commodity) to add to the account.
    ///
    /// This may be `None`, if it is the only element within a
    /// [Transaction](Transaction), which is None. If it is `None`,
    /// it's amount will be automatically calculated from the amounts
    /// in the other elements present in the transaction.
    pub amount: Option<Commodity>,

    /// The exchange rate to use for converting the amount in this element
    /// to a different [CommodityType](commodity::CommodityType).
    pub exchange_rate: Option<ExchangeRate>,
}

impl TransactionElement {
    /// Create a new [TransactionElement](TransactionElement).
    pub fn new(
        account_id: AccountID,
        amount: Option<Commodity>,
        exchange_rate: Option<ExchangeRate>,
    ) -> TransactionElement {
        TransactionElement {
            account_id,
            amount,
            exchange_rate,
        }
    }
}

/// A type of [Action](Action) to edit the
/// [AccountStatus](AccountStatus) of a given [Account](crate::Account)'s
/// [AccountState](super::AccountState).
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct EditAccountStatus {
    account_id: AccountID,
    newstatus: AccountStatus,
    date: NaiveDate,
}

impl EditAccountStatus {
    /// Create a new [EditAccountStatus](EditAccountStatus).
    pub fn new(
        account_id: AccountID,
        newstatus: AccountStatus,
        date: NaiveDate,
    ) -> EditAccountStatus {
        EditAccountStatus {
            account_id,
            newstatus,
            date,
        }
    }
}

impl fmt::Display for EditAccountStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Edit Account Status")
    }
}

impl<AT, ATV> Action<AT, ATV> for EditAccountStatus
where
    ATV: ActionTypeValueEnum<AT>,
{
    fn date(&self) -> NaiveDate {
        self.date
    }

    fn perform(&self, program_state: &mut ProgramState<AT, ATV>) -> Result<(), AccountingError> {
        let mut account_state = program_state
            .get_account_state_mut(&self.account_id)
            .unwrap();
        account_state.status = self.newstatus;
        Ok(())
    }
}

impl ActionTypeFor<ActionType> for EditAccountStatus {
    fn action_type(&self) -> ActionType {
        ActionType::EditAccountStatus
    }
}

/// A type of [Action](Action) to check and assert the balance of a
/// given [Account](crate::Account) in its [AccountStatus](AccountStatus) at
/// the beginning of the given date.
///
/// When running its [perform()](Action::perform()) method, if this
/// assertion fails, a [FailedBalanceAssertion](FailedBalanceAssertion)
/// will be recorded in the [ProgramState](ProgramState).
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct BalanceAssertion {
    account_id: AccountID,
    date: NaiveDate,
    expected_balance: Commodity,
}

impl BalanceAssertion {
    /// Create a new [BalanceAssertion](BalanceAssertion). The balance
    /// will be considered at the beginning of the provided `date`.
    pub fn new(
        account_id: AccountID,
        date: NaiveDate,
        expected_balance: Commodity,
    ) -> BalanceAssertion {
        BalanceAssertion {
            account_id,
            date,
            expected_balance,
        }
    }
}

impl fmt::Display for BalanceAssertion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Assert Account Balance")
    }
}

/// Records the failure of a [BalanceAssertion](BalanceAssertion) when
/// it is evaluated using its implementation of the
/// [Action::perform()](Action::perform()) method.
#[derive(Debug, Clone)]
pub struct FailedBalanceAssertion {
    pub assertion: BalanceAssertion,
    pub actual_balance: Commodity,
}

impl FailedBalanceAssertion {
    /// Create a new [FailedBalanceAssertion](FailedBalanceAssertion).
    pub fn new(assertion: BalanceAssertion, actual_balance: Commodity) -> FailedBalanceAssertion {
        FailedBalanceAssertion {
            assertion,
            actual_balance,
        }
    }
}

impl fmt::Display for FailedBalanceAssertion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Failed Account Balance Assertion")
    }
}

// When running this action's `perform()` method implementation, if
// this assertion fails, a [FailedBalanceAssertion](FailedBalanceAssertion)
// will be recorded in the [ProgramState](ProgramState).
impl<AT, ATV> Action<AT, ATV> for BalanceAssertion
where
    ATV: ActionTypeValueEnum<AT>,
{
    fn date(&self) -> NaiveDate {
        self.date
    }

    fn perform(&self, program_state: &mut ProgramState<AT, ATV>) -> Result<(), AccountingError> {
        let failed_assertion = match program_state.get_account_state(&self.account_id) {
            Some(state) => {
                if !state
                    .amount
                    .eq_approx(self.expected_balance, Commodity::default_epsilon())
                {
                    Some(FailedBalanceAssertion::new(self.clone(), state.amount))
                } else {
                    None
                }
            }
            None => {
                return Err(AccountingError::MissingAccountState(self.account_id));
            }
        };

        if let Some(failed_assertion) = failed_assertion {
            program_state.record_failed_balance_assertion(failed_assertion)
        }

        Ok(())
    }
}

impl ActionTypeFor<ActionType> for BalanceAssertion {
    fn action_type(&self) -> ActionType {
        ActionType::BalanceAssertion
    }
}

#[cfg(test)]
mod tests {
    use super::ActionType;
    use crate::{
        Account, AccountStatus, AccountingError, ActionTypeValue,
        BalanceAssertion, Program, ProgramState, Transaction,
    };
    use chrono::NaiveDate;
    use commodity::{Commodity, CommodityType};
    use rust_decimal::Decimal;
    use std::{collections::HashSet, rc::Rc};

    #[test]
    fn action_type_order() {
        let mut tested_types: HashSet<ActionType> = HashSet::new();

        let mut action_types_unordered: Vec<ActionType> = vec![
            ActionType::Transaction,
            ActionType::EditAccountStatus,
            ActionType::BalanceAssertion,
            ActionType::EditAccountStatus,
            ActionType::Transaction,
            ActionType::BalanceAssertion,
        ];

        let num_action_types = ActionType::iterator().count();

        action_types_unordered.iter().for_each(|action_type| {
            tested_types.insert(action_type.clone());
        });

        assert_eq!(num_action_types, tested_types.len());

        action_types_unordered.sort();

        let action_types_ordered: Vec<ActionType> = vec![
            ActionType::EditAccountStatus,
            ActionType::EditAccountStatus,
            ActionType::BalanceAssertion,
            ActionType::BalanceAssertion,
            ActionType::Transaction,
            ActionType::Transaction,
        ];

        assert_eq!(action_types_ordered, action_types_unordered);
    }

    #[test]
    fn balance_assertion() {
        let aud = Rc::from(CommodityType::from_currency_alpha3("AUD").unwrap());
        let account1 = Rc::from(Account::new_with_id(Some("Account 1"), aud.id, None));
        let account2 = Rc::from(Account::new_with_id(Some("Account 2"), aud.id, None));

        let date_1 = NaiveDate::from_ymd(2020, 01, 01);
        let date_2 = NaiveDate::from_ymd(2020, 01, 02);
        let actions: Vec<Rc<ActionTypeValue>> = vec![
            Rc::new(
                Transaction::new_simple::<String>(
                    None,
                    date_1.clone(),
                    account1.id,
                    account2.id,
                    Commodity::new(Decimal::new(100, 2), &*aud),
                    None,
                )
                .into(),
            ),
            // This assertion is expected to fail because it occurs at the start
            // of the day (before the transaction).
            Rc::new(
                BalanceAssertion::new(
                    account2.id,
                    date_1.clone(),
                    Commodity::new(Decimal::new(100, 2), &*aud),
                )
                .into(),
            ),
            // This assertion is expected to pass because it occurs at the end
            // of the day (after the transaction).
            Rc::new(
                BalanceAssertion::new(
                    account2.id,
                    date_2.clone(),
                    Commodity::new(Decimal::new(100, 2), &*aud),
                )
                .into(),
            ),
        ];

        let program = Program::new(actions);

        let accounts = vec![account1, account2];
        let mut program_state = ProgramState::new(&accounts, AccountStatus::Open);
        match program_state.execute_program(&program) {
            Err(AccountingError::BalanceAssertionFailed(failure)) => {
                assert_eq!(
                    Commodity::new(Decimal::new(0, 2), &*aud),
                    failure.actual_balance
                );
                assert_eq!(date_1, failure.assertion.date);
            }
            _ => panic!("Expected an AccountingError:BalanceAssertionFailed"),
        }

        assert_eq!(1, program_state.failed_balance_assertions.len());
    }
}

#[cfg(feature = "serde-support")]
#[cfg(test)]
mod serde_tests {
    use super::{BalanceAssertion, EditAccountStatus, Transaction};
    use crate::{AccountID, AccountStatus};
    use chrono::NaiveDate;
    use commodity::Commodity;
    use std::str::FromStr;

    #[test]
    fn edit_account_status_serde() {
        use serde_json;

        let json = r#"{
    "account_id": "TestAccount",
    "newstatus": "Open",
    "date": "2020-05-10"
}"#;
        let action: EditAccountStatus = serde_json::from_str(json).unwrap();

        let reference_action = EditAccountStatus::new(
            AccountID::from("TestAccount").unwrap(),
            AccountStatus::Open,
            NaiveDate::from_ymd(2020, 05, 10),
        );

        assert_eq!(action, reference_action);

        insta::assert_json_snapshot!(action);
    }

    #[test]
    fn balance_assertion_serde() {
        use serde_json;

        let json = r#"{
    "account_id": "TestAccount",
    "date": "2020-05-10",
    "expected_balance": {
        "value": "1.0",
        "type_id": "AUD"
    }
}"#;
        let action: BalanceAssertion = serde_json::from_str(json).unwrap();

        let reference_action = BalanceAssertion::new(
            AccountID::from("TestAccount").unwrap(),
            NaiveDate::from_ymd(2020, 05, 10),
            Commodity::from_str("1.0 AUD").unwrap(),
        );

        assert_eq!(action, reference_action);

        insta::assert_json_snapshot!(action);
    }

    #[cfg(feature = "serde-support")]
    #[test]
    fn transaction_serde() {
        use serde_json;

        let json = r#"{
    "description": "TestTransaction",
    "date": "2020-05-10",
    "elements": [
        {
            "account_id": "TestAccount1",
            "amount": {
                "value": "-1.0",
                "type_id": "AUD"
            }
        },
        {
            "account_id": "TestAccount2"
        }
    ]  
}"#;
        let action: Transaction = serde_json::from_str(json).unwrap();

        let reference_action = Transaction::new_simple(
            Some("TestTransaction"),
            NaiveDate::from_ymd(2020, 05, 10),
            AccountID::from("TestAccount1").unwrap(),
            AccountID::from("TestAccount2").unwrap(),
            Commodity::from_str("1.0 AUD").unwrap(),
            None,
        );

        assert_eq!(action, reference_action);

        insta::assert_json_snapshot!(action);
    }
}
