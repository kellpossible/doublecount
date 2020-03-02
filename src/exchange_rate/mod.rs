extern crate chrono;
extern crate rust_decimal;

#[cfg(feature = "serde-support")]
extern crate serde;
#[cfg(feature = "serde-support")]
extern crate serde_json;

extern crate thiserror;

use commodity::{Commodity, CurrencyCode};
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;

#[cfg(feature = "serde")]
use serde::Deserialize;

use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExchangeRateError {
    #[error("the currency {0} is not present in the exchange rate")]
    CurrencyNotPresent(CurrencyCode),
}

pub enum ExchangeRateSource {
    /// A local source
    Local,
    /// From the internet (string indicating the source)
    Internet(String),
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
#[derive(Debug, Clone)]
pub struct ExchangeRate {
    /// The datetime that this exchange rate represents
    pub date: Option<NaiveDate>,
    /// The datetime that this exchange rate was obtained.
    pub obtained_datetime: Option<DateTime<Utc>>,
    /// The base currency for the exchange rate
    pub base: Option<CurrencyCode>,
    /// Maps currency codes, to the conversion rate from that currency
    /// to the base currency.
    pub rates: HashMap<CurrencyCode, Decimal>,
}

impl ExchangeRate {
    pub fn get_rate(&self, currency_code: &CurrencyCode) -> Option<&Decimal> {
        self.rates.get(currency_code)
    }

    /// Convert the currency of a [Commodity](Commodity) from one currency to another
    /// using this [ExchangeRate](ExchangeRate).
    pub fn convert(
        &self,
        commodity: Commodity,
        target_currency: CurrencyCode,
    ) -> Result<Commodity, ExchangeRateError> {
        match self.base {
            // handle the situation where there is a base currency
            Some(base) => {
                if commodity.currency_code == base {
                    match self.get_rate(&target_currency) {
                        Some(rate) => {
                            return Ok(Commodity::new(rate * commodity.value, target_currency))
                        }
                        None => {}
                    };
                }

                if target_currency == base {
                    match self.get_rate(&commodity.currency_code) {
                        Some(rate) => {
                            return Ok(Commodity::new(commodity.value / rate, target_currency))
                        }
                        None => {}
                    };
                }
            }
            None => {}
        }

        // handle the situation where there is no base currency, or neither the commodity
        // currency or the target currency are the base currency.

        let commodity_rate = match self.get_rate(&commodity.currency_code) {
            Some(rate) => rate,
            None => {
                return Err(ExchangeRateError::CurrencyNotPresent(
                    commodity.currency_code,
                ))
            }
        };

        let target_rate = match self.get_rate(&target_currency) {
            Some(rate) => rate,
            None => return Err(ExchangeRateError::CurrencyNotPresent(target_currency)),
        };

        let value = (commodity.value / commodity_rate) * target_rate;
        return Ok(Commodity::new(value, target_currency));
    }
}

#[cfg(test)]
mod tests {
    use super::{Commodity, CurrencyCode, ExchangeRate};
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use std::collections::HashMap;
    use std::str::FromStr;

    #[cfg(feature = "serde-support")]
    #[test]
    fn test_deserialize() {
        use serde_json;

        let data = r#"
            {
                "date": "2020-02-07",
                "base": "AUD",
                "rates": {
                    "USD": 2.542,
                    "EU": "1.234"
                }
            }
            "#;

        let exchange_rate: ExchangeRate = serde_json::from_str(data).unwrap();
        let usd = CurrencyCode::from_str("USD").unwrap();
        let eu = CurrencyCode::from_str("EU").unwrap();

        assert_eq!(
            NaiveDate::from_ymd(2020, 02, 07),
            exchange_rate.date.unwrap()
        );
        assert_eq!("AUD", exchange_rate.base.unwrap());
        assert_eq!(
            Decimal::from_str("2.542").unwrap(),
            *exchange_rate.get_rate(&usd).unwrap()
        );
        assert_eq!(
            Decimal::from_str("1.234").unwrap(),
            *exchange_rate.get_rate(&eu).unwrap()
        );
    }

    #[test]
    fn convert_reference_rates() {
        let mut rates: HashMap<CurrencyCode, Decimal> = HashMap::new();
        let aud = CurrencyCode::from_str("AUD").unwrap();
        let nzd = CurrencyCode::from_str("NZD").unwrap();
        rates.insert(aud, Decimal::from_str("1.6417").unwrap());
        rates.insert(nzd, Decimal::from_str("1.7094").unwrap());

        let exchange_rate = ExchangeRate {
            date: Some(NaiveDate::from_ymd(2020, 02, 07)),
            base: None,
            obtained_datetime: None,
            rates,
        };

        {
            let start_commodity = Commodity::new(Decimal::from_str("10.0").unwrap(), aud);
            let converted_commodity = exchange_rate.convert(start_commodity, nzd);
            assert_eq!(
                Decimal::from_str("10.412377413656575501005055735").unwrap(),
                converted_commodity.unwrap().value
            );
        }

        {
            let start_commodity = Commodity::new(Decimal::from_str("10.0").unwrap(), nzd);
            let converted_commodity = exchange_rate.convert(start_commodity, aud);
            assert_eq!(
                Decimal::from_str("9.603954603954603954603954604").unwrap(),
                converted_commodity.unwrap().value
            );
        }
    }

    #[test]
    fn convert_base_rate() {
        let mut rates: HashMap<CurrencyCode, Decimal> = HashMap::new();
        let nok = CurrencyCode::from_str("NOK").unwrap();
        let usd = CurrencyCode::from_str("USD").unwrap();
        rates.insert(nok, Decimal::from_str("9.2691220713").unwrap());

        let exchange_rate = ExchangeRate {
            date: Some(NaiveDate::from_ymd(2020, 02, 07)),
            base: Some(usd),
            obtained_datetime: None,
            rates,
        };

        {
            let start_commodity = Commodity::new(Decimal::from_str("100.0").unwrap(), usd);
            let converted_commodity = exchange_rate.convert(start_commodity, nok);
            assert_eq!(
                Decimal::from_str("926.91220713000").unwrap(),
                converted_commodity.unwrap().value
            );
        }

        {
            let start_commodity = Commodity::new(Decimal::from_str("100.0").unwrap(), nok);
            let converted_commodity = exchange_rate.convert(start_commodity, usd);
            assert_eq!(
                Decimal::from_str("10.788508256853169187585300627").unwrap(),
                converted_commodity.unwrap().value
            );
        }
    }
}
