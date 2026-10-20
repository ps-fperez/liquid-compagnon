//! Budget : revenus (salaire, revenus ponctuels), charges fixes récurrentes, dépenses ponctuelles
//! et leurs catégories.

mod category;
mod charge;
mod expense;
mod income;

pub use category::Category;
pub use charge::{Periodicity, RecurringCharge};
pub use expense::Expense;
pub use income::{OneOffIncome, SalaryEntry};
