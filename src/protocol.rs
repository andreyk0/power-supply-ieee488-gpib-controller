//! AR488 serial protocol

use core::fmt::Write;

use heapless::{consts::*, ArrayLength, String};

use crate::prelude::AppError;

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
pub enum ChannelHeader {
    Vset,
    Iset,
    Vout,
    Iout,
    Out,
}

#[derive(Copy, Clone, Debug)]
pub enum Command {
    Vset { ch: Channel, val: f32 },
    Iset { ch: Channel, val: f32 },
    Out { ch: Channel, on: bool },
}

impl Command {
    pub fn append_to_str<S>(&self, buf: &mut String<S>) -> Result<(), AppError>
    where
        S: ArrayLength<u8>,
    {
        match self {
            Command::Vset { ch, val } => write!(buf, "VSET {} {:.3};", ch.to_str(), val)?,
            Command::Iset { ch, val } => write!(buf, "ISET {} {:.3};", ch.to_str(), val)?,
            Command::Out { ch, on } => {
                write!(buf, "OUT {} {};", ch.to_str(), if *on { "1" } else { "0" })?
            }
        }

        Ok(())
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Query {
    pub header: ChannelHeader,
    pub channel: Channel,
}

pub const QUERY_PING_LOOP: [Query; 18] = [
    Query {
        header: ChannelHeader::Vset,
        channel: Channel::Ch1,
    },
    Query {
        header: ChannelHeader::Vset,
        channel: Channel::Ch2,
    },
    Query {
        header: ChannelHeader::Vout,
        channel: Channel::Ch1,
    },
    Query {
        header: ChannelHeader::Vout,
        channel: Channel::Ch2,
    },
    Query {
        header: ChannelHeader::Iout,
        channel: Channel::Ch1,
    },
    Query {
        header: ChannelHeader::Iout,
        channel: Channel::Ch2,
    },
    Query {
        header: ChannelHeader::Iset,
        channel: Channel::Ch1,
    },
    Query {
        header: ChannelHeader::Iset,
        channel: Channel::Ch2,
    },
    Query {
        header: ChannelHeader::Vout,
        channel: Channel::Ch1,
    },
    Query {
        header: ChannelHeader::Vout,
        channel: Channel::Ch2,
    },
    Query {
        header: ChannelHeader::Iout,
        channel: Channel::Ch1,
    },
    Query {
        header: ChannelHeader::Iout,
        channel: Channel::Ch2,
    },
    Query {
        header: ChannelHeader::Out,
        channel: Channel::Ch1,
    },
    Query {
        header: ChannelHeader::Out,
        channel: Channel::Ch2,
    },
    Query {
        header: ChannelHeader::Vout,
        channel: Channel::Ch1,
    },
    Query {
        header: ChannelHeader::Vout,
        channel: Channel::Ch2,
    },
    Query {
        header: ChannelHeader::Iout,
        channel: Channel::Ch1,
    },
    Query {
        header: ChannelHeader::Iout,
        channel: Channel::Ch2,
    },
];

impl Query {
    pub fn to_str(&self) -> String<U8> {
        let mut s: String<U8> = String::new();

        let q = match self.header {
            ChannelHeader::Vset => "VSET",
            ChannelHeader::Iset => "ISET",
            ChannelHeader::Vout => "VOUT",
            ChannelHeader::Iout => "IOUT",
            ChannelHeader::Out => "OUT",
        };

        write!(s, "{}? {}", q, self.channel.to_str()).unwrap();
        s
    }

    pub fn write_serial_cmd_buf(&self, sbuf: &mut String<U32>) {
        sbuf.clear();
        write!(sbuf, "{}\r\n++read eoi\r\n", self.to_str()).unwrap();
    }
}
