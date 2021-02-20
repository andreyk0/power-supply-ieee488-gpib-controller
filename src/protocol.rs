//! AR488 serial protocol

use core::fmt::Write;

use heapless::{consts::*, String};

#[derive(Copy, Clone, Debug)]
pub enum Channel {
    Ch1,
    Ch2,
}

impl Channel {
    pub fn to_str(&self) -> &'static str {
        match self {
            Channel::Ch1 => "1",
            Channel::Ch2 => "2",
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Command {
    Vset { ch: Channel, val: f32 },
    Iset { ch: Channel, val: f32 },
}

#[derive(Copy, Clone, Debug)]
pub enum Query {
    Vset(Channel),
    Iset(Channel),
    Vout(Channel),
    Iout(Channel),
}

pub const QUERY_PING_LOOP: [Query; 8] = [
    Query::Vset(Channel::Ch1),
    Query::Iset(Channel::Ch1),
    Query::Vset(Channel::Ch2),
    Query::Iset(Channel::Ch2),
    Query::Vout(Channel::Ch1),
    Query::Iout(Channel::Ch1),
    Query::Vout(Channel::Ch2),
    Query::Iout(Channel::Ch2),
];

impl Query {
    pub fn to_str(&self) -> String<U8> {
        let mut s: String<U8> = String::new();

        let (q, ch) = match self {
            Query::Vset(ch) => ("VSET", ch),
            Query::Iset(ch) => ("ISET", ch),
            Query::Vout(ch) => ("VOUT", ch),
            Query::Iout(ch) => ("IOUT", ch),
        };

        write!(s, "{}? {}", q, ch.to_str()).unwrap();
        s
    }

    pub fn write_serial_cmd_buf(&self, sbuf: &mut String<U32>) {
        sbuf.clear();
        write!(sbuf, "{}\r\n++read eoi\r\n", self.to_str()).unwrap();
    }

    pub fn query_channel(&self) -> Option<Channel> {
        match self {
            Query::Vset(ch) => Some(*ch),
            Query::Iset(ch) => Some(*ch),
            Query::Vout(ch) => Some(*ch),
            Query::Iout(ch) => Some(*ch),
        }
    }
}
