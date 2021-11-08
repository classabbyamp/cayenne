use std::{cmp::Ordering, fmt::{self, Display}, str::FromStr};

/// represents a numerical value in a SPICE file.
///
/// primarily intended to be parsed from a string using [`Number::from_str()`].
/// Internally, the input is parsed to an [`f64`], so input rules of [`f64::from_str()`] apply
/// (except `inf`, `-inf`, and `NaN`).
///
/// Values can also be appended with SI prefixes to denote magnitude, instead of using `n.nnEnn` notation.
/// For example, `1.23k` would be parsed to `1230.0`. The following case-insensitive values are allowed:
///
/// | Symbol | Prefix | Equivalent Exponent |
/// |--------|--------|---------------------|
/// | T      | Tera   | `E+12`              |
/// | G      | Giga   | `E+09`              |
/// | X, Meg | Mega   | `E+06`              |
/// | K      | Kilo   | `E+03`              |
/// | M      | Milli  | `E-03`              |
/// | U      | Micro  | `E-06`              |
/// | N      | Nano   | `E-09`              |
/// | P      | Pico   | `E-12`              |
/// | F      | Femto  | `E-15`              |
///
/// [`f64::from_str()`]: https://doc.rust-lang.org/1.56.0/std/primitive.f64.html#method.from_str
// TODO: Fix `f64::from_str()` link (see rust-lang/rust#90703)
#[derive(Debug)]
pub struct Number {
    pub value: f64,
    pub raw: String,
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl Default for Number {
    fn default() -> Self {
        Number {
            value: 0.0,
            raw: String::from("0"),
        }
    }
}

impl Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl FromStr for Number {
    type Err = ParseNumberError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut raw = s.chars();
        let mut state = NumParseState::Start;
        let mut next_state: NumParseState;
        let mut c: char;
        let mut value_str = String::new();
        let mut mult = 1.0;

        'parse: loop {
            if let Some(ch) = raw.next() {
                c = ch;
            } else if value_str.len() > 0 {
                break 'parse;
            } else {
                return Err(ParseNumberError{kind: NumberErrorKind::Empty});
            }

            match state {
                NumParseState::Start | NumParseState::ExpStart => match c {
                    '+' | '-' | '0'..='9' => {
                        value_str.push(c);
                        if state == NumParseState::Start { next_state = NumParseState::Int; }
                            else { next_state = NumParseState::Exp; }
                    }
                    _ => return Err(ParseNumberError{kind: NumberErrorKind::Invalid}),
                },
                NumParseState::Int | NumParseState::Float => match c {
                    '0'..='9' => {
                        value_str.push(c);
                        next_state = NumParseState::Int;
                    }
                    '.' => match state {
                        NumParseState::Int => {
                            value_str.push(c);
                            next_state = NumParseState::Float;
                        }
                        NumParseState::Float | _ => return Err(ParseNumberError{kind: NumberErrorKind::Invalid}),
                    }
                    'e' | 'E' => {
                        value_str.push(c);
                        next_state = NumParseState::ExpStart;
                    }
                    _ if c.is_ascii_alphabetic() => { // unit multiplier
                        match c.to_ascii_uppercase() {
                            'T' => mult = 1e12, // Tera
                            'G' => mult = 1e9, // Giga
                            'X' => mult = 1e6, // Mega
                            'K' => mult = 1e3, // Kilo
                            'M' => { // Milli (m) or Mega (Meg)
                                if raw.take(2).collect::<String>().to_ascii_uppercase() == "EG" { mult = 1e6; }
                                    else { mult = 1e-3; }
                            }
                            'U' => mult = 1e-6, // Micro
                            'N' => mult = 1e-9, // Nano
                            'P' => mult = 1e-12, // Pico
                            'F' => mult = 1e-15, // Femto
                            _ => return Err(ParseNumberError{kind: NumberErrorKind::InvalidMult}),
                        }
                        break 'parse;
                    }
                    _ => return Err(ParseNumberError{kind: NumberErrorKind::Invalid}),
                },
                NumParseState::Exp => match c {
                    '0'..='9' => {
                        value_str.push(c);
                        next_state = NumParseState::Exp;
                    }
                    _ => break 'parse,
                },
            };

            state = next_state;
        }

        let value = match value_str.parse::<f64>() {
            Ok(v) => v * mult,
            Err(_) => return Err(ParseNumberError{kind: NumberErrorKind::Invalid})
        };

        Ok(Self{ value, raw: s.into() })
    }
}

#[derive(PartialEq)]
enum NumParseState {
    Start,
    Int,
    Float,
    ExpStart,
    Exp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNumberError {
    kind: NumberErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NumberErrorKind {
    Empty,
    Invalid,
    InvalidMult,
}

impl ParseNumberError {
    #[doc(hidden)]
    pub fn __description(&self) -> &str {
        match self.kind {
            NumberErrorKind::Empty => "cannot parse number from empty string",
            NumberErrorKind::Invalid => "invalid number",
            NumberErrorKind::InvalidMult => "invalid multiplier",
        }
    }
}

pub type Node = String;


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default() {
        assert_eq!(Number::default(), Number{value: 0.0, raw: String::from("0")})
    }

    #[test]
    fn int() {
        assert_eq!(
            Number::from_str("7343"),
            Ok( Number{ value: 7343.0, raw: String::from("7343") } )
        )
    }

    #[test]
    fn plus_int() {
        assert_eq!(
            Number::from_str("+123"),
            Ok( Number{ value: 123.0, raw: String::from("+123") } )
        )
    }

    #[test]
    fn minus_int() {
        assert_eq!(
            Number::from_str("-453"),
            Ok( Number{ value: -453.0, raw: String::from("-453") } )
        )
    }

    #[test]
    fn float() {
        assert_eq!(
            Number::from_str("1.23"),
            Ok( Number{ value: 1.23, raw: String::from("1.23") } )
        )
    }

    #[test]
    fn plus_float() {
        assert_eq!(
            Number::from_str("+87343.54"),
            Ok( Number{ value: 87343.54, raw: String::from("+87343.54") } )
        )
    }

    #[test]
    fn minus_float() {
        assert_eq!(
            Number::from_str("-8484.00927"),
            Ok( Number{ value: -8484.00927, raw: String::from("-8484.00927") } )
        )
    }

    #[test]
    fn plus_int_exp_lower() {
        assert_eq!(
            Number::from_str("+473e3"),
            Ok( Number{ value: 473e3, raw: String::from("+473e3") } )
        )
    }

    #[test]
    fn minus_int_exp_upper_plus() {
        assert_eq!(
            Number::from_str("-234E+7"),
            Ok( Number{ value: -234e7, raw: String::from("-234E+7") } )
        )
    }

    #[test]
    fn int_exp_lower_plus_leading_zeros() {
        assert_eq!(
            Number::from_str("34e+0007"),
            Ok( Number{ value: 34e7, raw: String::from("34e+0007") } )
        )
    }

    #[test]
    fn int_exp_upper_minus() {
        assert_eq!(
            Number::from_str("4E-2"),
            Ok( Number{ value: 4e-2, raw: String::from("4E-2") } )
        )
    }

    #[test]
    fn minus_int_exp_upper_minus_leading_zeros() {
        assert_eq!(
            Number::from_str("-4E-08"),
            Ok( Number{ value: -4e-8, raw: String::from("1.23") } )
        )
    }

    #[test]
    fn plus_float_exp_lower() {
        assert_eq!(
            Number::from_str("+4.73e3"),
            Ok( Number{ value: 4.73e3, raw: String::from("+4.73e3") } )
        )
    }

    #[test]
    fn minus_float_exp_upper_plus() {
        assert_eq!(
            Number::from_str("-23.4E+7"),
            Ok( Number{ value: -23.4e7, raw: String::from("-23.4E+7") } )
        )
    }

    #[test]
    fn float_exp_upper_plus() {
        assert_eq!(
            Number::from_str("10.34E+4"),
            Ok( Number{ value: 10.34e4, raw: String::from("10.34E+4") } )
        )
    }

    #[test]
    fn plus_int_with_unit_lower() {
        assert_eq!(
            Number::from_str("+123t"),
            Ok( Number{ value: 123e12, raw: String::from("+123t") } )
        )
    }

    #[test]
    fn minus_int_with_unit_upper() {
        assert_eq!(
            Number::from_str("-453X"),
            Ok( Number{ value: -453e6, raw: String::from("-453X") } )
        )
    }

    #[test]
    fn int_with_unit_meg() {
        assert_eq!(
            Number::from_str("7343Meg"),
            Ok( Number{ value: 7343e6, raw: String::from("7343Meg") } )
        )
    }

    #[test]
    fn float_with_unit_meg() {
        assert_eq!(
            Number::from_str("1.23Meg"),
            Ok( Number{ value: 1.23e6, raw: String::from("1.23Meg") } )
        )
    }

    #[test]
    fn plus_float_with_unit_upper() {
        assert_eq!(
            Number::from_str("+87343.54K"),
            Ok( Number{ value: 87343.54e3, raw: String::from("+87343.54K") } )
        )
    }

    #[test]
    fn minus_float_with_unit_lower() {
        assert_eq!(
            Number::from_str("-8484.00923m"),
            Ok( Number{ value: -8484.00923e-3, raw: String::from("-8484.00923m") } )
        )
    }

    #[test]
    fn float_with_unit_extra() {
        assert_eq!(
            Number::from_str("1.23pFarad"),
            Ok( Number{ value: 1.23e-12, raw: String::from("1.23pFarad") } )
        )
    }

    #[test]
    fn exp_and_unit() {
        assert_eq!(
            Number::from_str("123e3F"),
            Ok( Number{ value: 123e3, raw: String::from("123e3F") } )
        )
    }

    #[test]
    fn invalid_empty() {
        assert_eq!(
            Number::from_str(""),
            Err( ParseNumberError{ kind: NumberErrorKind::Empty } )
        )
    }

    #[test]
    fn invalid_multiple_points() {
        assert_eq!(
            Number::from_str("1.2.3"),
            Err( ParseNumberError{ kind: NumberErrorKind::Invalid } )
        )
    }

    #[test]
    fn invalid_chars1() {
        assert_eq!(
            Number::from_str("3-4"),
            Err( ParseNumberError{ kind: NumberErrorKind::Invalid } )
        )
    }

    #[test]
    fn invalid_chars2() {
        assert_eq!(
            Number::from_str("3+4"),
            Err( ParseNumberError{ kind: NumberErrorKind::Invalid } )
        )
    }

    #[test]
    fn invalid_chars3() {
        assert_eq!(
            Number::from_str("potato"),
            Err( ParseNumberError{ kind: NumberErrorKind::Invalid } )
        )
    }

    #[test]
    fn invalid_sign() {
        assert_eq!(
            Number::from_str("+-474.0"),
            Err( ParseNumberError{ kind: NumberErrorKind::Invalid } )
        )
    }

    #[test]
    fn invalid_mult() {
        assert_eq!(
            Number::from_str("474.0W"),
            Err( ParseNumberError{ kind: NumberErrorKind::InvalidMult } )
        )
    }
}
