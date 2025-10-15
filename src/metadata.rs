use std::str::FromStr;

use thiserror::Error;
use winnow::combinator::{alt, cut_err, eof, seq, terminated};
use winnow::error::{StrContext, StrContextValue};
use winnow::stream::AsChar;
use winnow::token::{rest, take_until, take_while};
use winnow::{ModalResult, Parser};

use crate::identifiers::NamedId;
use crate::metadata::api::models::minecraft::VersionId;

pub mod api;

#[derive(Error, Debug)]
#[error("invalid server kind: {value}")]
pub struct ServerKindParseError {
    value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ServerKind {
    #[default]
    Vanilla,
    Fabric,
    Forge,
    Neoforge,
    Paper,
}

impl FromStr for ServerKind {
    // TODO: custom error type
    type Err = ServerKindParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "vanilla" => Ok(ServerKind::Vanilla),
            "fabric" => Ok(ServerKind::Fabric),
            "forge" => Ok(ServerKind::Forge),
            "neoforge" => Ok(ServerKind::Neoforge),
            "paper" => Ok(ServerKind::Paper),
            _ => Err(ServerKindParseError {
                value: s.to_string(),
            }),
        }
    }
}

// TODO: move
#[derive(Debug, Clone)]
pub struct ServerSpec {
    version: VersionId,
    id: NamedId,
    server_type: ServerKind,
}

fn parse_server_spec(input: &mut &str) -> ModalResult<ServerSpec> {
    let mut version = alt((
        // empty input or empty version field
        eof.default_value(),
        ':'.default_value(),
        // version present
        cut_err(alt((terminated(take_until(1.., ':'), ':'), rest)).parse_to()).context(
            StrContext::Expected(StrContextValue::Description("valid version number")),
        ),
    ));

    let mut id = alt((
        eof.default_value(),
        ':'.default_value(),
        cut_err(
            terminated(
                take_while(1.., (AsChar::is_alphanum, '-', '_')),
                alt((eof, ":")),
            )
            .map(|s: &str| NamedId::new(s.to_string())),
        )
        .context(StrContext::Expected(StrContextValue::Description(
            "[a-zA-Z0-9_\\-]",
        ))),
    ));

    let mut server_type = alt((
        eof.default_value(),
        cut_err(rest.parse_to()).context(StrContext::Expected(StrContextValue::Description(
            "a valid server type",
        ))),
    ));

    seq!(ServerSpec {
        version: version,
        id: id,
        server_type: server_type
    })
    .parse_next(input)
}

impl FromStr for ServerSpec {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_server_spec
            .parse(s)
            .map_err(|e| anyhow::anyhow!("parsing server specification failed:\n{e}"))
    }
}

impl Default for ServerSpec {
    fn default() -> Self {
        ServerSpec {
            version: VersionId::default(),
            id: NamedId::new("unnamed".to_string()),
            server_type: ServerKind::Vanilla,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn parse_simple() {
        let latest = VersionId::default();

        let spec: ServerSpec = "1.20.1".parse().unwrap();
        dbg!(&spec);
        assert_eq!(spec.version.as_str(), "1.20.1");
        assert_eq!(spec.server_type, ServerKind::Vanilla);

        let spec: ServerSpec = "::forge".parse().unwrap();
        dbg!(&spec);
        assert_eq!(spec.version, latest);
        assert!(spec.id.to_string().starts_with("unnamed ("))
    }
}
