# Changelog

## v0.6.0

+ Renamed argument in `sum_account_states()`, `sum_currency` to
  `sum_commodity_type_id` to better match the recent changes in `commodity` library.

## v0.5.0

+ Updated `commodity` library dependency to `v0.3.0`, renamed some types
  which were changed for that version
+ `AccountingError::Currency` renamed to `AccountingError::Commodity`
