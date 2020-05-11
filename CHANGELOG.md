# Changelog

## v0.7.1

+ Fix building for default features accidentally including some items only needed for `serde-support`.

## v0.7.0

+ Complete `serde` serialization for `Program`, and the various `Action` implementations. [#2](https://github.com/kellpossible/doublecount/issues/2).
+ Refactor `Program` to use an enum `ActionTypeValue` instead of dynamic trait dispatch over `Action`. Programs using custom actions need to create their own implementation of `ActionTypeValueEnum` to provide store their actions given to `Program`.

## v0.6.2

+ Fix changelog fo `v0.6.1`

## v0.6.1

+ Bump `rust_decimal` dependency up to using generic version `1` to address [#5](https://github.com/kellpossible/doublecount/issues/5).
+ Update `Account#new()`, `Transaction#new()` and `Transaction#new_simple()` to use `Into<String>` to address [#4](https://github.com/kellpossible/doublecount/issues/4).

## v0.6.0

+ Renamed argument in `sum_account_states()`, `sum_currency` to
  `sum_commodity_type_id` to better match the recent changes in `commodity` library.

## v0.5.0

+ Updated `commodity` library dependency to `v0.3.0`, renamed some types
  which were changed for that version
+ `AccountingError::Currency` renamed to `AccountingError::Commodity`
