//! Read line from a buffer

use heapless::{ArrayLength, Vec};

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
