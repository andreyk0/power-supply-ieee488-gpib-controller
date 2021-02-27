//! Read line from a buffer

use core::str::FromStr;

use heapless::{ArrayLength, String, Vec};

use crate::prelude::*;

pub fn fill_until_eol<S1, S2>(line_buf: &mut Vec<u8, S1>, data_buf: &mut Vec<u8, S2>) -> bool
where
    S1: ArrayLength<u8>,
    S2: ArrayLength<u8>,
{
    let mut found = false;
    let mut n = 0usize;

    for b in data_buf.iter() {
        match line_buf.push(*b) {
            Ok(()) => {
                n += 1;
            }
            Err(_) => {
                break;
            }
        }
        if *b == b'\n' {
            found = true;
            break;
        }
    }

    if n > 0 {
        if data_buf.len() == n {
            data_buf.clear();
        } else {
            let left = data_buf.len() - n;
            for i in 0..left {
                data_buf[i] = data_buf[n + i]
            }
            data_buf.truncate(left)
        }
    }

    found
}

pub fn to_str_skip_whitespace<S>(line: &Vec<u8, S>, strbuf: &mut String<S>) -> Result<(), AppError>
where
    S: ArrayLength<u8>,
{
    strbuf.clear();

    for b in line {
        if b.is_ascii_whitespace() {
            continue;
        }
        strbuf.push(*b as char).map_err(|_| AppError::Duh)?;
    }

    Ok(())
}

#[inline]
pub fn parse_str<S, N>(str: &String<S>) -> Result<N, AppError>
where
    S: ArrayLength<u8>,
    N: FromStr,
{
    str.parse::<N>().map_err(|_| AppError::ParseError)
}
