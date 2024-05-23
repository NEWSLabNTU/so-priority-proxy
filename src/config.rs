use anyhow::{bail, ensure, Context};
use std::{
    fs::File,
    io::{prelude::*, BufReader},
    net::SocketAddr,
    path::Path,
    str::FromStr,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Config {
    pub maps: Vec<Map>,
}

impl Config {
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let reader = BufReader::new(File::open(path)?);
        let lines = reader.lines();
        let maps: Result<Vec<Map>, _> = lines.map(|line| anyhow::Ok(line?.parse()?)).collect();
        Ok(Self { maps: maps? })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Map {
    pub protocol: Protocol,
    pub src: SocketAddr,
    pub dst: SocketAddr,
    pub priority: u8,
}

impl FromStr for Map {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let format_err =
            || format!("expect '<protocol> | <priority> | <src> -> <dst>', but get '{text}'");

        let mut tokens = text.split_ascii_whitespace();
        let Some(protocol) = tokens.next() else {
            bail!(format_err());
        };
        let Some("|") = tokens.next() else {
            bail!(format_err());
        };
        let Some(priority) = tokens.next() else {
            bail!(format_err());
        };
        let Some("|") = tokens.next() else {
            bail!(format_err());
        };
        let Some(src) = tokens.next() else {
            bail!(format_err());
        };
        let Some("->") = tokens.next() else {
            bail!(format_err());
        };
        let Some(dst) = tokens.next() else {
            bail!(format_err());
        };
        ensure!(tokens.next().is_none(), format_err());

        let protocol: Protocol = protocol
            .parse()
            .with_context(|| format!("'{protocol}' is not a valid protocol name"))
            .with_context(format_err)?;
        let src: SocketAddr = src
            .parse()
            .with_context(|| format!("'{src}' is not a valid socket address"))
            .with_context(format_err)?;
        let dst: SocketAddr = dst
            .parse()
            .with_context(|| format!("'{src}' is not a valid socket address"))
            .with_context(format_err)?;
        let priority: u8 = priority
            .parse()
            .with_context(|| format!("'{priority}' is not a valid priority number"))
            .with_context(format_err)?;

        Ok(Self {
            protocol,
            src,
            dst,
            priority,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    Tcp,
    Udp,
}

impl FromStr for Protocol {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let protocol = match text {
            "tcp" => Self::Tcp,
            "udp" => Self::Udp,
            _ => bail!("invalid protocol '{text}'"),
        };
        Ok(protocol)
    }
}
