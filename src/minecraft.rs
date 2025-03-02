pub mod api;

use std::str::FromStr;

use derive_more::derive::{Display, From};
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use winnow::ascii::digit1;
use winnow::combinator::{alt, eof, fail, opt, peek, preceded};
use winnow::error::{StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::seq;
use winnow::stream::AsChar;
use winnow::token::take_while;

#[derive(Debug, SerializeDisplay, DeserializeFromStr, PartialEq, Clone)]
pub struct ReleaseVersionNumber {
    // u8 is reasonable for Minecraft specifically; this can be easily changed
    major: u8,
    minor: u8,
    patch: u8,
}

impl std::fmt::Display for ReleaseVersionNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.patch == 0 {
            write!(f, "{}.{}", self.major, self.minor)
        } else {
            write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
        }
    }
}

impl FromStr for ReleaseVersionNumber {
    // TODO: replace error types with custom error or color_eyre::Report
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        release_version.parse(s).map_err(|e| e.to_string())
    }
}

fn release_version(i: &mut &str) -> winnow::Result<ReleaseVersionNumber> {
    let (major, minor, patch) = seq!(
        digit1.parse_to().context(StrContext::Label("major")),
        _: '.',
        digit1.parse_to().context(StrContext::Label("minor")),
        opt(preceded(
            '.',
            digit1.parse_to()
                .context(StrContext::Label("patch"))
                .context(StrContext::Expected(StrContextValue::Description("patch"))),
        )),
        _: peek(alt((eof, "-"))).context(StrContext::Label("eof or pre-release")),
    )
    .parse_next(i)?;

    Ok(ReleaseVersionNumber {
        major,
        minor,
        patch: patch.unwrap_or(0),
    })
}

#[derive(Debug, Display, SerializeDisplay, DeserializeFromStr, PartialEq, Clone)]
#[display("{release}-{pre_release}")]
pub struct PreReleaseVersionNumber {
    release: ReleaseVersionNumber,
    pre_release: String,
}

impl FromStr for PreReleaseVersionNumber {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        pre_release_version.parse(s).map_err(|e| e.to_string())
    }
}

fn pre_release_version(i: &mut &str) -> winnow::Result<PreReleaseVersionNumber> {
    let (rv, pre_s, pre_n) = seq!(
        release_version.context(StrContext::Label("release version")),
        _: '-'.context(StrContext::Label("pre-release separator"))
            .context(StrContext::Expected(StrContextValue::CharLiteral('-'))),
        alt(("pre", "rc")).context(StrContext::Label("pre-release type")),
        digit1.context(StrContext::Label("pre-release number")),
    )
    .parse_next(i)?;

    Ok(PreReleaseVersionNumber {
        release: rv,
        // avoids a format! call (and double allocation?)
        pre_release: [pre_s, pre_n].concat(),
    })
}

#[derive(Debug, Display, SerializeDisplay, DeserializeFromStr, PartialEq, Clone)]
#[display("{year}w{week:02}{snapshot}")]
pub struct SnapshotVersionNumber {
    year: u8,
    week: u8,
    // usually single letter starting with 'a', except april fools snapshots
    snapshot: char, // TODO: is this a good idea?
}

impl FromStr for SnapshotVersionNumber {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        snapshot_version.parse(s).map_err(|e| e.to_string())
    }
}

fn snapshot_version(i: &mut &str) -> winnow::Result<SnapshotVersionNumber> {
    seq!(SnapshotVersionNumber {
        year: take_while(2, AsChar::is_dec_digit)
            .parse_to()
            .context(StrContext::Label("year"))
            .context(StrContext::Expected(StrContextValue::Description("two digit year"))),
        _: 'w',
        week: take_while(2, AsChar::is_dec_digit)
            .parse_to()
            .context(StrContext::Label("week"))
            .context(StrContext::Expected(StrContextValue::Description("two digit week"))),
        snapshot: take_while(1, 'a'..='z')
            .parse_to()
            .context(StrContext::Label("snapshot"))
            .context(StrContext::Expected(StrContextValue::Description("lowercase letter"))),
        _: eof
    })
    .parse_next(i)
}

/// All-encompassing version number type, including versions that don't fit the three standard formats (as [`VersionNumber::NonStandard`])
#[derive(Debug, Display, From, Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum VersionNumber {
    Release(ReleaseVersionNumber),
    PreRelease(PreReleaseVersionNumber),
    Snapshot(SnapshotVersionNumber),
    // this captures old_beta, old_alpha, some 1.14 snapshots, and april fools snapshots
    NonStandard(String),
}

/// Parses any version number string into a `VersionNumber` with the appropriate variant
impl FromStr for VersionNumber {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        version_number.parse(s).map_err(|e| e.to_string())
    }
}

fn version_number(i: &mut &str) -> winnow::Result<VersionNumber> {
    alt((
        // pre-release contains a release version, so it must be checked first
        pre_release_version
            .map(VersionNumber::PreRelease)
            .context(StrContext::Label("pre-release version")),
        release_version
            .map(VersionNumber::Release)
            .context(StrContext::Label("release version")),
        snapshot_version
            .map(VersionNumber::Snapshot)
            .context(StrContext::Label("snapshot version")),
        take_while(4.., (AsChar::is_alphanum, '.', '-', '_', ' '))
            .map(|s: &str| VersionNumber::NonStandard(s.into()))
            .context(StrContext::Label("non-standard version"))
            .context(StrContext::Expected(StrContextValue::Description(
                "[a-zA-Z0-9._- ]",
            ))),
        fail.context(StrContext::Expected(StrContextValue::Description(
            "version number (4 or more characters)",
        ))),
    ))
    .parse_next(i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macros::assert_matches;

    /// Test that a given string parses to the expected result or panics
    macro_rules! test_parse {
        ($name:ident: $parser:ident($input:expr) => panic) => {
            #[test]
            #[should_panic]
            fn $name() {
                let _result = $parser.parse($input).unwrap();
            }
        };
        ($name:ident: $parser:ident($input:expr) => $expected:pat) => {
            #[test]
            fn $name() {
                let result = $parser.parse($input).unwrap();
                assert_matches!(result, $expected);
            }
        };
    }

    /// Test that a type can be parsed and then serialized back to the original string
    ///
    /// NOTE: only useful for canonical representations of the version numbers, i.e. as found in the manifest
    macro_rules! test_roundtrip {
        ($name:ident: $parser:ident($input:expr)) => {
            #[test]
            fn $name() {
                let result = $parser.parse($input).unwrap();
                assert_eq!(result.to_string(), $input);
            }
        };
    }

    test_parse!(parse_release1: release_version("1.2.3") => ReleaseVersionNumber { major: 1, minor: 2, patch: 3 });
    test_parse!(parse_release2: release_version("1.2") => ReleaseVersionNumber { major: 1, minor: 2, patch: 0 });
    test_parse!(parse_release3: release_version("1") => panic);
    test_parse!(parse_release4: release_version("1.2.") => panic);
    test_parse!(parse_release5: release_version("1.2.3.") => panic);
    test_parse!(parse_release6: release_version("0.01.2") => ReleaseVersionNumber { major: 0, minor: 1, patch: 2 });
    test_parse!(parse_release7: release_version("10.12.24") => ReleaseVersionNumber { major: 10, minor: 12, patch: 24 });
    test_parse!(parse_release8: release_version("1.0.256") => panic);
    test_roundtrip!(release_roundtrip: release_version("1.2.3"));

    test_parse!(parse_pre_release1: pre_release_version("1.2.3-pre1") => PreReleaseVersionNumber { release: ReleaseVersionNumber { major: 1, minor: 2, patch: 3 }, pre_release: _ });
    test_parse!(parse_pre_release2: pre_release_version("1.2.3-rc") => panic);
    test_parse!(parse_pre_release3: pre_release_version("1.2.3-prea") => panic);
    test_parse!(parse_pre_release4: pre_release_version("1.2-pre99") => PreReleaseVersionNumber { release: ReleaseVersionNumber { major: 1, minor: 2, patch: 0 }, pre_release: _ });
    test_roundtrip!(pre_release_roundtrip: pre_release_version("1.2.3-pre1"));

    test_parse!(parse_snapshot1: snapshot_version("13w24a") => SnapshotVersionNumber { year: 13, week: 24, snapshot: 'a' });
    test_parse!(parse_snapshot2: snapshot_version("24w11") => panic);
    test_parse!(parse_snapshot3: snapshot_version("22w43a1") => panic);
    test_parse!(parse_snapshot4: snapshot_version("1w05a") => panic);
    test_parse!(parse_snapshot5: snapshot_version("12w4a") => panic);
    test_parse!(parse_snapshot6: snapshot_version("15w081") => panic);
    test_parse!(parse_snapshot7: snapshot_version("17a22b") => panic);
    test_parse!(parse_snapshot8: snapshot_version("14w38.") => panic);
    test_parse!(parse_snapshot9: snapshot_version("16w19ab") => panic);
    test_roundtrip!(snapshot_roundtrip: snapshot_version("13w04a"));

    test_parse!(parse_version1: version_number("1.2.3") => VersionNumber::Release(ReleaseVersionNumber { major: 1, minor: 2, patch: 3 }));
    test_parse!(parse_version2: version_number("1.2.3-pre1") => VersionNumber::PreRelease(PreReleaseVersionNumber { release: ReleaseVersionNumber { major: 1, minor: 2, patch: 3 }, pre_release: _ }));
    test_parse!(parse_version3: version_number("13w24a") => VersionNumber::Snapshot(SnapshotVersionNumber { year: 13, week: 24, snapshot: 'a' }));
    test_parse!(parse_version4: version_number("foobar") => VersionNumber::NonStandard(_));
    test_parse!(parse_version5: version_number("") => panic);
    test_parse!(parse_version6: version_number("24w14potato") => VersionNumber::NonStandard(_));
    test_parse!(parse_version7: version_number("1.14.2 Pre-Release 4") => VersionNumber::NonStandard(_));
    test_roundtrip!(version_roundtrip: version_number("foobar"));
}
