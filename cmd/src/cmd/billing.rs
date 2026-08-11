//! Shared billing report types and formatting

use std::collections::BTreeSet;
use std::io::{self, Write as _};
use std::str::FromStr;

use chrono::{Datelike, NaiveDate};
use clap::ValueEnum;
use eyre::{eyre, Result, WrapErr};
use rust_decimal::Decimal;
use serde::Serialize;

/// Output format for a cloud billing report
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum BillingOutput {
    /// Human-readable text
    #[default]
    Human,
    /// Stable machine-readable JSON
    Json,
}

/// Inclusive dates covered by a billing report
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct BillingPeriod {
    pub(crate) start: NaiveDate,
    pub(crate) usage_through: NaiveDate,
}

impl BillingPeriod {
    pub(crate) fn current_month(today: NaiveDate) -> Result<Self> {
        let start = today
            .with_day(1)
            .ok_or_else(|| eyre!("calculate the first day of the current month"))?;

        Ok(Self {
            start,
            usage_through: today,
        })
    }

    pub(crate) fn end_exclusive(self) -> Result<NaiveDate> {
        self.usage_through
            .succ_opt()
            .ok_or_else(|| eyre!("calculate the end of the current billing period"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Money {
    value: Decimal,
    currency: String,
}

impl Money {
    pub(crate) fn new(amount: &str, currency: &str, context: &str) -> Result<Self> {
        let value = parse_decimal(amount)
            .wrap_err_with(|| format!("{context} has an invalid monetary amount"))?
            .normalize();
        let currency = currency.trim();
        if currency.len() != 3 || !currency.bytes().all(|byte| byte.is_ascii_uppercase()) {
            return Err(eyre!("{context} has an invalid currency code"));
        }

        Ok(Self {
            value,
            currency: currency.to_string(),
        })
    }

    pub(crate) fn zero(currency: impl Into<String>) -> Self {
        Self {
            value: Decimal::ZERO,
            currency: currency.into(),
        }
    }

    pub(crate) fn add(&mut self, other: &Self, context: &str) -> Result<()> {
        if self.currency != other.currency {
            return Err(eyre!(
                "{context} uses mixed billing currencies: {} and {}",
                self.currency,
                other.currency
            ));
        }
        self.value = self
            .value
            .checked_add(other.value)
            .ok_or_else(|| eyre!("{context} monetary total is too large"))?
            .normalize();

        Ok(())
    }

    pub(crate) fn is_zero(&self) -> bool {
        self.value.is_zero()
    }

    pub(crate) fn currency(&self) -> &str {
        &self.currency
    }

    pub(crate) fn json(&self) -> JsonMoney {
        JsonMoney {
            value: self.value,
            currency: self.currency.clone(),
        }
    }
}

impl std::fmt::Display for Money {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.currency == "USD" {
            return write!(formatter, "${:.2} USD", self.value);
        }

        write!(formatter, "{:.2} {}", self.value, self.currency)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct JsonMoney {
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    value: Decimal,
    currency: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LabelFilter(Vec<String>);

impl LabelFilter {
    pub(crate) fn new(values: Vec<String>) -> Result<Self> {
        let mut labels = BTreeSet::new();
        for value in values {
            let value = value.trim();
            if value.is_empty() {
                return Err(eyre!("billing filters must not be empty"));
            }

            labels.insert(value.to_lowercase());
        }

        Ok(Self(labels.into_iter().collect()))
    }

    pub(crate) fn matches(&self, label: &str) -> bool {
        self.0.is_empty() || self.0.binary_search(&label.to_lowercase()).is_ok()
    }
}

pub(crate) fn write_stdout(rendered: &str) -> Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(rendered.as_bytes())?;

    Ok(())
}

fn parse_decimal(value: &str) -> Result<Decimal, rust_decimal::Error> {
    if value.trim() != value || value.is_empty() {
        return Decimal::from_str("");
    }

    Decimal::from_str(value).or_else(|_| Decimal::from_scientific(value))
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::{BillingPeriod, LabelFilter, Money};

    #[test]
    fn current_month_includes_today() {
        let period =
            BillingPeriod::current_month(NaiveDate::from_ymd_opt(2026, 8, 10).unwrap()).unwrap();

        assert_eq!(period.start, NaiveDate::from_ymd_opt(2026, 8, 1).unwrap());
        assert_eq!(
            period.end_exclusive().unwrap(),
            NaiveDate::from_ymd_opt(2026, 8, 11).unwrap()
        );
    }

    #[test]
    fn money_addition_is_exact() {
        let mut total = Money::new("0.1", "USD", "first value").unwrap();
        total
            .add(&Money::new("0.2", "USD", "second value").unwrap(), "total")
            .unwrap();

        assert_eq!(total.to_string(), "$0.30 USD");
        assert_eq!(
            serde_json::to_string(&total.json()).unwrap(),
            r#"{"value":0.3,"currency":"USD"}"#
        );
    }

    #[test]
    fn label_filters_are_trimmed_exact_and_case_insensitive() {
        let filter =
            LabelFilter::new(vec![" Droplets ".to_string(), "DROPLETS".to_string()]).unwrap();

        assert!(filter.matches("droplets"));
        assert!(!filter.matches("GPU Droplets"));
    }
}
