//! Godot string conversion functions.

use crate::util::NPeekable;
use std::iter::once;
use unicode_xid::UnicodeXID;

pub trait GodotUnicodeXID {
    fn is_godot_xid_start(&self) -> bool;
}

impl GodotUnicodeXID for char {
    fn is_godot_xid_start(&self) -> bool {
        *self == '_' || self.is_xid_start()
    }
}

pub fn hex_to_int(s: &str) -> i64 {
    if s.is_empty() {
        return 0;
    }

    let mut chars = NPeekable::new(s.chars());

    let sign = if chars.peek(0) == Some('-') {
        chars.next();
        -1i64
    } else {
        1
    };

    if chars.peek(0) == Some('0') && chars.peek(1).map(|c| c.to_ascii_lowercase()) == Some('x') {
        chars.next();
        chars.next();
    }

    let mut result = 0i64;

    for ch in chars {
        let ch = ch.to_ascii_lowercase();

        let n = if ch.is_ascii_digit() {
            ch as u32 - '0' as u32
        } else if ch > 'a' && ch <= 'f' {
            ch as u32 - 'a' as u32 + 10
        } else {
            panic!(
                "Invalid hexadecimal notation character \"{}\" (U+{:04X}) in string \"{}\"",
                ch, ch as u32, s
            );
        };

        result = result
            .checked_mul(16)
            .and_then(|c| c.checked_add(n as i64))
            .unwrap_or_else(|| {
                panic!(
                    "Cannot represent {} as a 64-bit signed integer, since the value is too {}.",
                    s,
                    if sign == 1 { "large" } else { "small" }
                )
            });
    }

    result * sign
}

pub fn bin_to_int(s: &str) -> i64 {
    if s.is_empty() {
        return 0;
    }

    let mut chars = NPeekable::new(s.chars());

    let sign = if chars.peek(0) == Some('-') {
        chars.next();
        -1i64
    } else {
        1
    };

    if s.len() > 2
        && chars.peek(0) == Some('0')
        && chars.peek(1).map(|c| c.to_ascii_lowercase()) == Some('b')
    {
        chars.next();
        chars.next();
    }

    let mut result = 0i64;

    for ch in chars {
        let ch = ch.to_ascii_lowercase();

        let n = if ch == '0' || ch == '1' {
            ch as u32 - '0' as u32
        } else {
            // XXX: Why is this function lenient? wtf
            return 0;
        };

        result = result
            .checked_mul(2)
            .and_then(|c| c.checked_add(n as i64))
            .unwrap_or_else(|| {
                panic!(
                    "Cannot represent {} as a 64-bit signed integer, since the value is too {}.",
                    s,
                    if sign == 1 { "large" } else { "small" }
                )
            });
    }

    result * sign
}

pub fn to_int(s: &str) -> i64 {
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    enum Reading {
        Sign,
        Int,
        Done,
    }

    if s.is_empty() {
        return 0;
    }

    let mut integer = 0i64;
    let mut sign = 1i64;

    let mut reading = Reading::Sign;
    let mut iter = s.chars().peekable();

    while let Some(ch) = iter.peek() {
        let ch = *ch;

        match reading {
            Reading::Sign => {
                if ch.is_ascii_digit() {
                    reading = Reading::Int;
                    continue;
                } else if ch == '-' {
                    sign = -1;
                    reading = Reading::Int;
                } else if ch == '+' {
                    sign = 1;
                    reading = Reading::Int;
                } else {
                    // XXX: This seems like it should explode?
                }
            }
            Reading::Int => {
                if ch.is_ascii_digit() {
                    let n = ch as u32 - '0' as u32;

                    integer = integer
                        .checked_mul(10)
                        .and_then(|c| c.checked_add(n as i64))
                        .unwrap_or_else(|| {
                            panic!(
                                "Cannot represent {} as a 64-bit signed integer, since the value is too {}.",
                                s,
                                if sign == 1 { "large" } else { "small" }
                            )
                        });
                } else {
                    reading = Reading::Done;
                }
            }
            Reading::Done => {}
        }

        iter.next();
    }

    sign * integer
}

pub fn to_float(s: &str) -> f64 {
    if s.is_empty() {
        0.0
    } else {
        built_in_strtod(s)
    }
}

fn built_in_strtod(
    /* A decimal ASCII floating-point number,
     * optionally preceded by white space. Must
     * have form "-I.FE-X", where I is the integer
     * part of the mantissa, F is the fractional
     * part of the mantissa, and X is the
     * exponent. Either of the signs may be "+",
     * "-", or omitted. Either I or F may be
     * omitted, or both. The decimal point isn't
     * necessary unless F is present. The "E" may
     * actually be an "e". E and X may both be
     * omitted (but not just one). */
    s: &str,
    /* If non-nullptr, store terminating Cacter's
     * address here. */
    // C **endPtr = nullptr
) -> f64 {
    /* Largest possible base 10 exponent. Any
     * exponent larger than this will already
     * produce underflow or overflow, so there's
     * no need to worry about additional digits. */
    const MAX_EXPONENT: i32 = 511;

    /* Table giving binary powers of 10. Entry
     * is 10^2^i. Used to convert decimal
     * exponents into floating-point numbers. */
    const POWERS_OF_TEN: &[f64] = &[
        10., 100., 1.0e4, 1.0e8, 1.0e16, 1.0e32, 1.0e64, 1.0e128, 1.0e256,
    ];

    let chs = s.chars().chain(once('\0')).collect::<Vec<_>>();

    /*
     * Strip off leading blanks and check for a sign.
     */

    let mut p = 0usize;

    while chs[p] == ' ' || chs[p] == '\t' || chs[p] == '\n' {
        p += 1;
    }

    let sign = if chs[p] == '-' {
        p += 1;
        -1.0
    } else {
        if chs[p] == '+' {
            p += 1;
        }

        1.0
    };

    /*
     * Count the number of digits in the mantissa (including the decimal
     * point), and also locate the decimal point.
     */

    let mut mant_size = 0i32;
    let mut dec_pt = None;
    loop {
        let ch = chs[p];

        if !ch.is_ascii_digit() {
            if ch != '.' || dec_pt.is_some() {
                break;
            }

            dec_pt = Some(mant_size);
        }

        p += 1;
        mant_size += 1;
    }

    /*
     * Now suck up the digits in the mantissa. Use two integers to collect 9
     * digits each (this is faster than using floating-point). If the mantissa
     * has more than 18 digits, ignore the extras, since they can't affect the
     * value anyway.
     */
    let p_exp = p;
    p -= mant_size as usize;

    let dec_pt = match dec_pt {
        Some(dec_pt) => {
            mant_size -= 1; /* One of the digits was the point. */
            dec_pt
        }
        None => mant_size,
    };

    let frac_exp = if mant_size > 18 {
        mant_size = 18;
        dec_pt - 18
    } else {
        dec_pt - mant_size
    };

    let mut fraction = if mant_size == 0 {
        // p = 0;
        return sign * 0.0;
    } else {
        let mut frac_1 = 0;
        while mant_size > 9 {
            let mut ch = chs[p];
            p += 1;

            if ch == '.' {
                ch = chs[p];
                p += 1;
            }

            frac_1 = 10 * frac_1 + (ch as u32 - '0' as u32);
            mant_size -= 1;
        }

        let mut frac_2 = 0;
        while mant_size > 0 {
            let mut ch = chs[p];
            p += 1;

            if ch == '.' {
                ch = chs[p];
                p += 1;
            }

            frac_2 = 10 * frac_2 + (ch as u32 - '0' as u32);
            mant_size -= 1;
        }

        (1.0e9 * frac_1 as f64) + frac_2 as f64
    };

    /*
     * Skim off the exponent.
     */
    p = p_exp;
    let mut exp = 0;

    let mut exp_sign = false;

    if chs[p] == 'E' || chs[p] == 'e' {
        p += 1;

        if chs[p] == '-' {
            exp_sign = true;
            p += 1;
        } else {
            if chs[p] == '+' {
                p += 1;
            }

            exp_sign = false;
        }

        if !chs[p].is_ascii_digit() {
            // p = p_exp;
            return sign * fraction;
        }

        while chs[p].is_ascii_digit() {
            exp = exp * 10 + (chs[p] as i32 - '0' as i32);
            p += 1;
        }
    }

    if exp_sign {
        exp = frac_exp - exp;
    } else {
        exp += frac_exp;
    }

    /*
     * Generate a floating-point number that represents the exponent. Do this
     * by processing the exponent one bit at a time to combine many powers of
     * 2 of 10. Then combine the exponent with the fraction.
     */

    if exp < 0 {
        exp_sign = true;
        exp = -exp;
    } else {
        exp_sign = false;
    }

    if exp > MAX_EXPONENT {
        exp = MAX_EXPONENT;
        // WARN_PRINT("Exponent too high");
    }

    let mut dbl_exp = 1.0;

    let mut d = 0;

    while exp != 0 {
        if (exp & 1) != 0 {
            dbl_exp *= POWERS_OF_TEN[d];
        }

        exp >>= 1;
        d += 1;
    }

    if exp_sign {
        fraction /= dbl_exp;
    } else {
        fraction *= dbl_exp;
    }

    sign * fraction
}

pub fn is_godot_whitespace(ch: char, counts_zwsp_as_whitespace: bool) -> bool {
    let ch = ch as u32;

    ch == 0x20
        || ch == 0x00a0
        || ch == 0x1680
        || (0x2000..=0x200a).contains(&ch)
        || ch == 0x202f
        || ch == 0x205f
        || ch == 0x3000
        || ch == 0x2028
        || ch == 0x2029
        || (0x0009..=0x000d).contains(&ch)
        || ch == 0x0085
        || (counts_zwsp_as_whitespace && ch == 0x200b)
}

#[cfg(test)]
mod tests {
    use crate::string::to_float;

    #[test]
    fn test_to_float() {
        assert_eq!(to_float("."), 0.0);
        assert_eq!(to_float(".5"), 0.5);
        assert_eq!(to_float("-.2"), -0.2);
        assert_eq!(to_float("1.5"), 1.5);
        assert_eq!(to_float("7e10"), 7e10);
        assert_eq!(to_float("3.8e10"), 3.8e10);
        assert_eq!(to_float("6.9e+4"), 6.9e4);
        assert_eq!(to_float("8.4e-15"), 8.4e-15);
        assert_eq!(to_float("-17.6e-8"), -17.6e-8);
    }
}
