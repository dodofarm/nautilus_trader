// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2025 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  You may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

//! Represents a price in a market.

use std::{
    cmp::Ordering,
    fmt::{Debug, Display},
    hash::{Hash, Hasher},
    ops::{Add, AddAssign, Deref, Mul, Neg, Sub, SubAssign},
    str::FromStr,
};

#[cfg(feature = "high-precision")]
use nautilus_core::correctness::check_positive_i128;
#[cfg(not(feature = "high-precision"))]
use nautilus_core::correctness::check_positive_i64;
use nautilus_core::{
    correctness::{check_in_range_inclusive_f64, FAILED},
    parsing::precision_from_str,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize};
use thousands::Separable;

use super::fixed::{check_fixed_precision, FIXED_PRECISION, FIXED_SCALAR};
#[cfg(feature = "high-precision")]
use super::fixed::{f64_to_fixed_i128, fixed_i128_to_f64, PRECISION_DIFF_SCALAR};
#[cfg(not(feature = "high-precision"))]
use super::fixed::{f64_to_fixed_i64, fixed_i64_to_f64};

#[cfg(feature = "high-precision")]
pub type PriceRaw = i128;
#[cfg(not(feature = "high-precision"))]
pub type PriceRaw = i64;

/// The maximum raw price integer value.
#[no_mangle]
pub static PRICE_RAW_MAX: PriceRaw = (PRICE_MAX * FIXED_SCALAR) as PriceRaw;

/// The minimum raw price integer value.
#[no_mangle]
pub static PRICE_RAW_MIN: PriceRaw = (PRICE_MIN * FIXED_SCALAR) as PriceRaw;

/// The sentinel value for an unset or null price.
pub const PRICE_UNDEF: PriceRaw = PriceRaw::MAX;

/// The sentinel value for an error or invalid price.
pub const PRICE_ERROR: PriceRaw = PriceRaw::MIN;

/// The maximum valid price value which can be represented.
#[cfg(feature = "high-precision")]
pub const PRICE_MAX: f64 = 17_014_118_346_046.0;
#[cfg(not(feature = "high-precision"))]
pub const PRICE_MAX: f64 = 9_223_372_036.0;

/// The minimum valid price value which can be represented.
#[cfg(feature = "high-precision")]
pub const PRICE_MIN: f64 = -17_014_118_346_046.0;
#[cfg(not(feature = "high-precision"))]
pub const PRICE_MIN: f64 = -9_223_372_036.0;

/// The sentinel `Price` representing errors (this will be removed when Cython is gone).
pub const ERROR_PRICE: Price = Price {
    raw: 0,
    precision: 255,
};

#[cfg(feature = "high-precision")]
pub fn check_positive_price(value: PriceRaw, param: &str) -> anyhow::Result<()> {
    check_positive_i128(value, param)
}

#[cfg(not(feature = "high-precision"))]
pub fn check_positive_price(value: PriceRaw, param: &str) -> anyhow::Result<()> {
    check_positive_i64(value, param)
}

#[cfg(feature = "high-precision")]
/// The raw i64 price has already been scaled by 10^9. Further scale
/// it by the difference to FIXED_PRECISION to make it high-precision raw price.
pub fn decode_raw_price_i64(value: i64) -> PriceRaw {
    value as PriceRaw * PRECISION_DIFF_SCALAR as PriceRaw
}

#[cfg(not(feature = "high-precision"))]
pub fn decode_raw_price_i64(value: i64) -> PriceRaw {
    value
}

/// Represents a price in a market.
///
/// The number of decimal places may vary. For certain asset classes, prices may
/// have negative values. For example, prices for options instruments can be
/// negative under certain conditions.
///
/// Handles up to {FIXED_PRECISION} decimals of precision.
///
///  - `PRICE_MAX` = {PRICE_MAX}
///  - `PRICE_MIN` = {PRICE_MIN}
#[repr(C)]
#[derive(Clone, Copy, Default, Eq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.model")
)]
pub struct Price {
    /// Represents the raw fixed-point value, with `precision` defining the number of decimal places.
    pub raw: PriceRaw,
    /// The number of decimal places, with a maximum of {FIXED_PRECISION}.
    pub precision: u8,
}

impl Price {
    /// Creates a new [`Price`] instance with correctness checking.
    ///
    /// # Errors
    ///
    /// This function returns an error:
    /// - If `value` is invalid outside the representable range [{PRICE_MIN}, {PRICE_MAX}].
    /// - If `precision` is invalid outside the representable range [0, {FIXED_PRECISION}].
    ///
    /// # Notes
    ///
    /// PyO3 requires a `Result` type for proper error handling and stacktrace printing in Python.
    pub fn new_checked(value: f64, precision: u8) -> anyhow::Result<Self> {
        check_in_range_inclusive_f64(value, PRICE_MIN, PRICE_MAX, "value")?;
        check_fixed_precision(precision)?;

        #[cfg(feature = "high-precision")]
        let raw = f64_to_fixed_i128(value, precision);
        #[cfg(not(feature = "high-precision"))]
        let raw = f64_to_fixed_i64(value, precision);

        Ok(Self { raw, precision })
    }

    /// Creates a new [`Price`] instance.
    ///
    /// # Panics
    ///
    /// This function panics:
    /// - If a correctness check fails. See [`Price::new_checked`] for more details.
    pub fn new(value: f64, precision: u8) -> Self {
        Self::new_checked(value, precision).expect(FAILED)
    }

    /// Creates a new [`Price`] instance from the given `raw` fixed-point value and `precision`.
    ///
    /// # Panics
    ///
    /// This function panics:
    /// - If a correctness check fails. See [`Price::new_checked`] for more details.
    pub fn from_raw(raw: PriceRaw, precision: u8) -> Self {
        check_fixed_precision(precision).expect(FAILED);
        Self { raw, precision }
    }

    /// Creates a new [`Price`] instance with a value of zero with the given `precision`.
    ///
    /// # Panics
    ///
    /// This function panics:
    /// - If a correctness check fails. See [`Price::new_checked`] for more details.
    #[must_use]
    pub fn zero(precision: u8) -> Self {
        check_fixed_precision(precision).expect(FAILED);
        Self { raw: 0, precision }
    }

    /// Creates a new [`Price`] instance with the maximum representable value with the given `precision`.
    ///
    /// # Panics
    ///
    /// This function panics:
    /// - If a correctness check fails. See [`Price::new_checked`] for more details.
    #[must_use]
    pub fn max(precision: u8) -> Self {
        check_fixed_precision(precision).expect(FAILED);
        Self {
            raw: PRICE_RAW_MAX,
            precision,
        }
    }

    /// Creates a new [`Price`] instance with the minimum representable value with the given `precision`.
    ///
    /// # Panics
    ///
    /// This function panics:
    /// - If a correctness check fails. See [`Price::new_checked`] for more details.
    #[must_use]
    pub fn min(precision: u8) -> Self {
        check_fixed_precision(precision).expect(FAILED);
        Self {
            raw: PRICE_RAW_MIN,
            precision,
        }
    }

    /// Returns `true` if the value of this instance is undefined.
    #[must_use]
    pub fn is_undefined(&self) -> bool {
        self.raw == PRICE_UNDEF
    }

    /// Returns `true` if the value of this instance is zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.raw == 0
    }

    /// Returns the value of this instance as an `f64`.
    #[must_use]
    #[cfg(feature = "high-precision")]
    pub fn as_f64(&self) -> f64 {
        fixed_i128_to_f64(self.raw)
    }

    #[cfg(not(feature = "high-precision"))]
    pub fn as_f64(&self) -> f64 {
        fixed_i64_to_f64(self.raw)
    }

    /// Returns the value of this instance as a `Decimal`.
    #[must_use]
    pub fn as_decimal(&self) -> Decimal {
        // Scale down the raw value to match the precision
        let rescaled_raw =
            self.raw / PriceRaw::pow(10, u32::from(FIXED_PRECISION - self.precision));
        #[allow(clippy::unnecessary_cast)] // Required for precision modes
        Decimal::from_i128_with_scale(rescaled_raw as i128, u32::from(self.precision))
    }

    /// Returns a formatted string representation of this instance.
    #[must_use]
    pub fn to_formatted_string(&self) -> String {
        format!("{self}").separate_with_underscores()
    }
}

impl FromStr for Price {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let float_from_input = value
            .replace('_', "")
            .parse::<f64>()
            .map_err(|e| format!("Error parsing `input` string '{value}' as f64: {e}"))?;

        Self::new_checked(float_from_input, precision_from_str(value)).map_err(|e| e.to_string())
    }
}

impl<T: AsRef<str>> From<T> for Price {
    fn from(value: T) -> Self {
        Self::from_str(value.as_ref()).expect(FAILED)
    }
}

impl From<Price> for f64 {
    fn from(price: Price) -> Self {
        price.as_f64()
    }
}

impl From<&Price> for f64 {
    fn from(price: &Price) -> Self {
        price.as_f64()
    }
}

impl Hash for Price {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl PartialEq for Price {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl PartialOrd for Price {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }

    fn lt(&self, other: &Self) -> bool {
        self.raw.lt(&other.raw)
    }

    fn le(&self, other: &Self) -> bool {
        self.raw.le(&other.raw)
    }

    fn gt(&self, other: &Self) -> bool {
        self.raw.gt(&other.raw)
    }

    fn ge(&self, other: &Self) -> bool {
        self.raw.ge(&other.raw)
    }
}

impl Ord for Price {
    fn cmp(&self, other: &Self) -> Ordering {
        self.raw.cmp(&other.raw)
    }
}

impl Deref for Price {
    type Target = PriceRaw;

    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

impl Neg for Price {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self {
            raw: -self.raw,
            precision: self.precision,
        }
    }
}

impl Add for Price {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        assert!(
            self.precision >= rhs.precision,
            "Precision mismatch: cannot add precision {} to precision {} (precision loss)",
            rhs.precision,
            self.precision,
        );
        Self {
            raw: self
                .raw
                .checked_add(rhs.raw)
                .expect("Overflow occurred when adding `Price`"),
            precision: self.precision,
        }
    }
}

impl Sub for Price {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        assert!(
            self.precision >= rhs.precision,
            "Precision mismatch: cannot subtract precision {} from precision {} (precision loss)",
            rhs.precision,
            self.precision,
        );
        Self {
            raw: self
                .raw
                .checked_sub(rhs.raw)
                .expect("Underflow occurred when subtracting `Price`"),
            precision: self.precision,
        }
    }
}

impl AddAssign for Price {
    fn add_assign(&mut self, other: Self) {
        assert!(
            self.precision >= other.precision,
            "Precision mismatch: cannot add precision {} to precision {} (precision loss)",
            other.precision,
            self.precision,
        );
        self.raw = self
            .raw
            .checked_add(other.raw)
            .expect("Overflow occurred when adding `Price`");
    }
}

impl SubAssign for Price {
    fn sub_assign(&mut self, other: Self) {
        assert!(
            self.precision >= other.precision,
            "Precision mismatch: cannot subtract precision {} from precision {} (precision loss)",
            other.precision,
            self.precision,
        );
        self.raw = self
            .raw
            .checked_sub(other.raw)
            .expect("Underflow occurred when subtracting `Price`");
    }
}

impl Add<f64> for Price {
    type Output = f64;
    fn add(self, rhs: f64) -> Self::Output {
        self.as_f64() + rhs
    }
}

impl Sub<f64> for Price {
    type Output = f64;
    fn sub(self, rhs: f64) -> Self::Output {
        self.as_f64() - rhs
    }
}

impl Mul<f64> for Price {
    type Output = f64;
    fn mul(self, rhs: f64) -> Self::Output {
        self.as_f64() * rhs
    }
}

impl Debug for Price {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}({:.*})",
            stringify!(Price),
            self.precision as usize,
            self.as_f64()
        )
    }
}

impl Display for Price {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.*}", self.precision as usize, self.as_f64())
    }
}

impl Serialize for Price {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Price {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let price_str: &str = Deserialize::deserialize(_deserializer)?;
        let price: Self = price_str.into();
        Ok(price)
    }
}

////////////////////////////////////////////////////////////////////////////////
// Tests
////////////////////////////////////////////////////////////////////////////////
#[cfg(not(feature = "high-precision"))]
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use float_cmp::approx_eq;
    use rstest::rstest;
    use rust_decimal_macros::dec;

    use super::*;

    #[rstest]
    #[should_panic(expected = "Condition failed: `precision` exceeded maximum `FIXED_PRECISION`")]
    fn test_invalid_precision_new() {
        // Precision out of range for fixed
        let _ = Price::new(1.0, 10);
    }

    #[rstest]
    #[should_panic(expected = "Condition failed: `precision` exceeded maximum `FIXED_PRECISION`")]
    fn test_invalid_precision_from_raw() {
        // Precision out of range for fixed
        let _ = Price::from_raw(1, 10);
    }

    #[rstest]
    #[should_panic(expected = "Condition failed: `precision` exceeded maximum `FIXED_PRECISION`")]
    fn test_invalid_precision_max() {
        // Precision out of range for fixed
        let _ = Price::max(10);
    }

    #[rstest]
    #[should_panic(expected = "Condition failed: `precision` exceeded maximum `FIXED_PRECISION`")]
    fn test_invalid_precision_min() {
        // Precision out of range for fixed
        let _ = Price::min(10);
    }

    #[rstest]
    #[should_panic(expected = "Condition failed: `precision` exceeded maximum `FIXED_PRECISION`")]
    fn test_invalid_precision_zero() {
        // Precision out of range for fixed
        let _ = Price::zero(10);
    }

    #[rstest]
    fn test_new() {
        let price = Price::new(0.00812, 8);
        assert_eq!(price, price);
        assert_eq!(price.raw, 8_120_000);
        assert_eq!(price.precision, 8);
        assert_eq!(price.as_f64(), 0.00812);
        assert_eq!(price.to_string(), "0.00812000");
        assert!(!price.is_zero());
        assert_eq!(price.as_decimal(), dec!(0.00812000));
        assert!(approx_eq!(
            f64,
            price.as_f64(),
            0.00812,
            epsilon = 0.000_001
        ));
    }

    #[rstest]
    fn test_with_maximum_value() {
        let price = Price::new(PRICE_MAX, 9);
        assert_eq!(price.raw, 9_223_372_036_000_000_000);
        assert_eq!(price.as_decimal(), dec!(9223372036));
        assert_eq!(price.to_string(), "9223372036.000000000");
    }

    #[rstest]
    fn test_with_minimum_positive_value() {
        let price = Price::new(0.000_000_001, 9);
        assert_eq!(price.raw, 1);
        assert_eq!(price.as_decimal(), dec!(0.000000001));
        assert_eq!(price.to_string(), "0.000000001");
    }

    // #[rstest]
    // fn test_with_minimum_value() {
    //     let price = Price::new(PRICE_MIN, 9);
    //     assert_eq!(price.raw, -9_223_372_036_000_000_000);
    //     assert_eq!(price.as_decimal(), dec!(-9223372036));
    //     assert_eq!(price.to_string(), "-9223372036.000000000");
    //     assert_eq!(price.to_formatted_string(), "-9_223_372_036.000000000");
    // }
    //
    // #[rstest]
    // fn test_max() {
    //     let price = Price::max(9);
    //     assert_eq!(price.raw, 9_223_372_036_000_000_000);
    //     assert_eq!(price.as_decimal(), dec!(9223372036));
    //     assert_eq!(price.to_string(), "9223372036.000000000");
    //     assert_eq!(price.to_formatted_string(), "9_223_372_036.000000000");
    // }
    //
    // #[rstest]
    // fn test_min() {
    //     let price = Price::min(9);
    //     assert_eq!(price.raw, -9_223_372_036_000_000_000);
    //     assert_eq!(price.as_decimal(), dec!(-9223372036));
    //     assert_eq!(price.to_string(), "-9223372036.000000000");
    // }

    #[rstest]
    fn test_undefined() {
        let price = Price::from_raw(PRICE_UNDEF, 0);
        assert_eq!(price.raw, PRICE_UNDEF);
        assert!(price.is_undefined());
    }

    #[rstest]
    fn test_zero() {
        let price = Price::zero(0);
        assert_eq!(price.raw, 0);
        assert_eq!(price.to_string(), "0");
        assert!(price.is_zero());
    }

    #[rstest]
    fn test_is_zero() {
        let price = Price::new(0.0, 8);
        assert_eq!(price, price);
        assert_eq!(price.raw, 0);
        assert_eq!(price.precision, 8);
        assert_eq!(price.as_f64(), 0.0);
        assert_eq!(price.to_string(), "0.00000000");
        assert!(price.is_zero());
    }

    #[rstest]
    fn test_precision() {
        let price = Price::new(1.001, 2);
        assert_eq!(price.raw, 1_000_000_000);
        assert_eq!(price.to_string(), "1.00");
    }

    #[rstest]
    fn test_new_from_str() {
        let price: Price = "0.00812000".into();
        assert_eq!(price, price);
        assert_eq!(price.raw, 8_120_000);
        assert_eq!(price.precision, 8);
        assert_eq!(price.as_f64(), 0.00812);
        assert_eq!(price.to_string(), "0.00812000");
    }

    #[rstest]
    fn test_from_str_valid_input() {
        let input = "10.5";
        let expected_price = Price::new(10.5, precision_from_str(input));
        let result = Price::from(input);
        assert_eq!(result, expected_price);
    }

    #[rstest]
    fn test_from_str_invalid_input() {
        let input = "invalid";
        let result = Price::from_str(input);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_equality() {
        assert_eq!(Price::from("1.0"), Price::from("1.0"));
        assert_eq!(Price::from("1.0"), Price::from("1.0"));
        assert_ne!(Price::from("1.1"), Price::from("1.0"));
        assert!(Price::from("1.0") <= Price::from("1.0"));
        assert!(Price::from("1.1") > Price::from("1.0"));
        assert!(Price::from("1.0") >= Price::from("1.0"));
        assert!(Price::from("1.0") >= Price::from("1.0"));
        assert!(Price::from("1.0") >= Price::from("1.0"));
        assert!(Price::from("0.9") < Price::from("1.0"));
        assert!(Price::from("0.9") <= Price::from("1.0"));
        assert!(Price::from("0.9") <= Price::from("1.0"));
    }

    #[rstest]
    fn test_add() {
        let price1 = Price::new(1.000, 3);
        let price2 = Price::new(1.011, 3);
        let price3 = price1 + price2;
        assert_eq!(price3.raw, Price::from("2.011").raw);
    }

    #[rstest]
    fn test_sub() {
        let price1 = Price::new(1.011, 3);
        let price2 = Price::new(1.000, 3);
        let price3 = price1 - price2;
        assert_eq!(price3.raw, Price::from("0.011").raw);
    }

    #[rstest]
    fn test_add_assign() {
        let mut price = Price::new(1.000, 3);
        price += Price::new(1.011, 3);
        assert_eq!(price.raw, Price::from("2.011").raw);
    }

    #[rstest]
    fn test_sub_assign() {
        let precision = 3;
        let a = 1.000;
        let b = 0.011;
        let mut price = Price::new(a, precision);
        price -= Price::new(b, precision);
        assert_eq!(price, Price::new(a - b, 3));
    }

    #[rstest]
    fn test_mul() {
        let price1 = Price::new(1.000, 3);
        let price2 = Price::new(1.011, 3);
        let result = price1 * price2.into();
        assert!(approx_eq!(f64, result, 1.011, epsilon = 0.000_001));
    }

    #[rstest]
    fn test_debug() {
        let price = Price::from("44.12");
        let result = format!("{price:?}");
        assert_eq!(result, "Price(44.12)");
    }

    #[rstest]
    fn test_display() {
        let price = Price::from("44.12");
        let result = format!("{price}");
        assert_eq!(result, "44.12");
    }
}
