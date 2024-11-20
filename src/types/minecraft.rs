use std::str::FromStr;

use derive_more::derive::Display;
use winnow::ascii::digit1;
use winnow::combinator::{alt, fail, opt, preceded};
use winnow::error::{StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::seq;
use winnow::stream::AsChar;
use winnow::token::take_while;

#[derive(Debug)]
struct ReleaseVersionNumber {
    // u8 is reasonable for Minecraft specifically; this can be easily changed
    major: u8,
    minor: u8,
    patch: u8,
}

impl std::fmt::Display for ReleaseVersionNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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

fn release_version(i: &mut &str) -> PResult<ReleaseVersionNumber> {
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
    )
    .parse_next(i)?;

    Ok(ReleaseVersionNumber {
        major,
        minor,
        patch: patch.unwrap_or(0),
    })
}

#[derive(Debug, Display)]
#[display("{release}-{pre_release}")]
struct PreReleaseVersionNumber {
    release: ReleaseVersionNumber,
    pre_release: String,
}

impl FromStr for PreReleaseVersionNumber {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        pre_release_version.parse(s).map_err(|e| e.to_string())
    }
}

fn pre_release_version(i: &mut &str) -> PResult<PreReleaseVersionNumber> {
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
        pre_release: format!("{}{}", pre_s, pre_n),
    })
}

#[derive(Debug, Display)]
enum VersionNumber {
    Release(ReleaseVersionNumber),
    PreRelease(PreReleaseVersionNumber),
    Other(String),
}

impl FromStr for VersionNumber {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        version_number.parse(s).map_err(|e| e.to_string())
    }
}

fn version_number(i: &mut &str) -> PResult<VersionNumber> {
    // NOTE: winnow seems to be greedy/doesn't backtrack in some cases, so we need to try the most specific parsers first.
    // in practice, this means we need to try pre-release before release (which it contains)
    alt((
        pre_release_version
            .map(VersionNumber::PreRelease)
            .context(StrContext::Label("pre-release version")),
        release_version
            .map(VersionNumber::Release)
            .context(StrContext::Label("release version")),
        take_while(4.., (AsChar::is_alphanum, '.', '-', '_', ' '))
            .map(|s: &str| VersionNumber::Other(s.into()))
            .context(StrContext::Label("other version")),
        fail.context(StrContext::Expected(StrContextValue::Description(
            "version number",
        ))),
    ))
    .parse_next(i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macros::assert_matches;

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

    // NOTE: only useful for canonical representations of the version numbers, i.e. as found in the manifest
    macro_rules! test_bijective {
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
    test_bijective!(release_bijective: release_version("1.2.3"));

    test_parse!(parse_pre_release1: pre_release_version("1.2.3-pre1") => PreReleaseVersionNumber { release: ReleaseVersionNumber { major: 1, minor: 2, patch: 3 }, pre_release: _ });
    test_parse!(parse_pre_release2: pre_release_version("1.2.3-rc") => panic);
    test_parse!(parse_pre_release3: pre_release_version("1.2.3-prea") => panic);
    test_parse!(parse_pre_release4: pre_release_version("1.2-pre99") => PreReleaseVersionNumber { release: ReleaseVersionNumber { major: 1, minor: 2, patch: 0 }, pre_release: _ });
    test_bijective!(pre_release_bijective: pre_release_version("1.2.3-pre1"));

    test_parse!(parse_version1: version_number("1.2.3") => VersionNumber::Release(ReleaseVersionNumber { major: 1, minor: 2, patch: 3 }));
    test_parse!(parse_version2: version_number("1.2.3-pre1") => VersionNumber::PreRelease(PreReleaseVersionNumber { release: ReleaseVersionNumber { major: 1, minor: 2, patch: 3 }, pre_release: _ }));
    test_parse!(parse_version3: version_number("") => panic);
    test_bijective!(version_bijective: version_number("foobar"));
}
