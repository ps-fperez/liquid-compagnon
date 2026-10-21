//! Budget : revenus (salaire, revenus ponctuels), charges fixes récurrentes, dépenses ponctuelles
//! et leurs catégories.

mod calculator;
mod category;
mod charge;
mod expense;
mod income;

pub use calculator::BudgetCalculator;
pub use category::Category;
pub use charge::{Debit, Periodicity, RecurringCharge};
pub use expense::Expense;
pub use income::{OneOffIncome, SalaryEntry};
