use super::{AccountID, AccountStatus, AccountingError, ProgramState};
use chrono::NaiveDate;
use commodity::exchange_rate::ExchangeRate;
use commodity::Commodity;
use rust_decimal::{prelude::Zero, Decimal};
use std::fmt;
use std::rc::Rc;
use std::slice;

#[cfg(feature = "serde-support")]
use serde::{Deserialize, Serialize};

/// A representation of what type of [Action](Action) is being performed.
#[derive(PartialEq, Eq, Debug, PartialOrd, Ord, Hash, Clone)]
pub enum ActionType {
    /// An [Action](Action) to edit the status of an [Account](Account).
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
    /// A [Action](Action) to perform a transaction between [Account](Account)s.
    /// Represented by the [Transaction](Transaction) struct.
    Transaction,
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

/// Represents an action which can modify [ProgramState](ProgramState).
pub trait Action: fmt::Display + fmt::Debug {
    /// The date/time (in the account history) that the action was performed.
    fn date(&self) -> NaiveDate;

    /// Perform the action to mutate the [ProgramState](ProgramState).
    fn perform(&self, program_state: &mut ProgramState) -> Result<(), AccountingError>;

    /// What type of action is being performed.
    fn action_type(&self) -> ActionType;
}

/// A way to sort [Action](Action)s by their date, and then by the
/// priority of their [ActionType](ActionType).
///
/// # Example
/// ```
/// use doublecount::{Action, ActionOrder};
/// use std::rc::Rc;
///
/// let mut actions: Vec<Rc<dyn Action>> = Vec::new();
///
/// // let's pretend we created and added
/// // some actions to the actions vector
///
/// // sort the actions using this order
/// actions.sort_by_key(|a| ActionOrder(a.clone()));
/// ```
pub struct ActionOrder(pub Rc<dyn Action>);

impl PartialEq for ActionOrder {
    fn eq(&self, other: &ActionOrder) -> bool {
        self.0.action_type() == other.0.action_type() && self.0.date() == other.0.date()
    }
}

impl Eq for ActionOrder {}

impl PartialOrd for ActionOrder {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0
            .date()
            .partial_cmp(&other.0.date())
            .map(|date_order| date_order.then(self.0.action_type().cmp(&other.0.action_type())))
    }
}

impl Ord for ActionOrder {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .date()
            .cmp(&other.0.date())
            .then(self.0.action_type().cmp(&other.0.action_type()))
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
#[derive(Debug, Clone)]
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
    pub fn new(
        description: Option<String>,
        date: NaiveDate,
        elements: Vec<TransactionElement>,
    ) -> Transaction {
        Transaction {
            description,
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
    /// use commodity::{Currency, Commodity};
    /// use chrono::Local;
    /// use std::str::FromStr;
    ///
    /// let aud = Rc::from(Currency::from_alpha3("AUD").unwrap());
    ///
    /// let account1 = Rc::from(Account::new(Some("Account 1"), aud.code, None));
    /// let account2 = Rc::from(Account::new(Some("Account 2"), aud.code, None));
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
    pub fn new_simple(
        description: Option<&str>,
        date: NaiveDate,
        from_account: AccountID,
        to_account: AccountID,
        amount: Commodity,
        exchange_rate: Option<ExchangeRate>,
    ) -> Transaction {
        Transaction::new(
            description.map(|s| String::from(s)),
            date,
            vec![
                TransactionElement::new(from_account, Some(amount.neg()), exchange_rate.clone()),
                TransactionElement::new(to_account, None, exchange_rate),
            ],
        )
    }

    /// Get the [TransactionElement](TransactionElement) associated with the given [Account](Account)'s id.
    pub fn get_element(&self, account_id: &AccountID) -> Option<&TransactionElement> {
        self.elements.iter().find(|e| &e.account_id == account_id)
    }
}

impl fmt::Display for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Transaction")
    }
}

impl Action for Transaction {
    fn date(&self) -> NaiveDate {
        self.date
    }

    fn perform(&self, program_state: &mut ProgramState) -> Result<(), AccountingError> {
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

        let sum_currency_code = match empty_amount_element {
            Some(empty_i) => {
                let empty_element = self.elements.get(empty_i).unwrap();

                match program_state.get_account(&empty_element.account_id) {
                    Some(account) => account.currency_code,
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
                    Some(account) => account.currency_code,
                    None => return Err(AccountingError::MissingAccountState(account_id)),
                }
            }
        };

        let mut sum = Commodity::new(Decimal::zero(), sum_currency_code);

        let mut modified_elements = self.elements.clone();

        // Calculate the sum of elements (not including the empty element if there is one)
        for (i, element) in self.elements.iter().enumerate() {
            match empty_amount_element {
                Some(empty_i) => {
                    if i != empty_i {
                        //TODO: perform currency conversion here if required
                        sum = match sum.add(&element.amount.as_ref().unwrap()) {
                            Ok(value) => value,
                            Err(error) => return Err(AccountingError::Currency(error)),
                        }
                    }
                }
                None => {}
            }
        }

        // Calculate the value to use for the empty element (negate the sum of the other elements)
        match empty_amount_element {
            Some(empty_i) => {
                let modified_emtpy_element: &mut TransactionElement =
                    modified_elements.get_mut(empty_i).unwrap();
                let negated_sum = sum.neg();
                modified_emtpy_element.amount = Some(negated_sum.clone());

                sum = match sum.add(&negated_sum) {
                    Ok(value) => value,
                    Err(error) => return Err(AccountingError::Currency(error)),
                }
            }
            None => {}
        };

        if sum.value != Decimal::zero() {
            return Err(AccountingError::InvalidTransaction(
                self.clone(),
                String::from("sum of transaction elements does not equal zero"),
            ));
        }

        for transaction in &modified_elements {
            let mut account_state = program_state
                .get_account_state_mut(&transaction.account_id)
                .expect(
                    format!(
                        "unable to find state for account with id: {} please ensure this account was added to the program state before execution.",
                        transaction.account_id
                    )
                    .as_ref(),
                );

            match account_state.status {
                AccountStatus::Closed => Err(AccountingError::InvalidAccountStatus {
                    account_id: transaction.account_id,
                    status: account_state.status,
                }),
                _ => Ok(()),
            }?;

            // TODO: perform the currency conversion using the exchange rate (if present)

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
                    return Err(AccountingError::Currency(err));
                }
            }
        }

        return Ok(());
    }

    fn action_type(&self) -> ActionType {
        ActionType::Transaction
    }
}

/// An element of a [Transaction](Transaction).
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
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
    /// to a different [Currency](commodity::Currency)
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
/// [AccountStatus](AccountStatus) of a given [Account](Account)'s
/// [AccountState](super::AccountState).
// #[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
#[derive(Debug)]
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

impl Action for EditAccountStatus {
    fn date(&self) -> NaiveDate {
        self.date
    }

    fn perform(&self, program_state: &mut ProgramState) -> Result<(), AccountingError> {
        let mut account_state = program_state
            .get_account_state_mut(&self.account_id)
            .unwrap();
        account_state.status = self.newstatus;
        return Ok(());
    }

    fn action_type(&self) -> ActionType {
        ActionType::EditAccountStatus
    }
}

/// A type of [Action](Action) to check and assert the balance of a
/// given [Account](Account) in its [AccountStatus](AccountStatus) at
/// the beginning of the given date.
///
/// When running its [perform()](Action::perform()) method, if this
/// assertion fails, a [FailedBalanceAssertion](FailedBalanceAssertion)
/// will be recorded in the [ProgramState](ProgramState).
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
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
impl Action for BalanceAssertion {
    fn date(&self) -> NaiveDate {
        self.date
    }

    fn perform(&self, program_state: &mut ProgramState) -> Result<(), AccountingError> {
        match program_state.get_account_state(&self.account_id) {
            Some(state) => {
                if state
                    .amount
                    .eq_approx(self.expected_balance, Commodity::default_epsilon())
                {
                } else {
                }
            }
            None => {
                return Err(AccountingError::MissingAccountState(
                    self.account_id,
                ));
            }
        }

        return Ok(());
    }

    fn action_type(&self) -> ActionType {
        ActionType::BalanceAssertion
    }
}

#[cfg(test)]
mod tests {
    use super::ActionType;
    use std::collections::HashSet;

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
}
