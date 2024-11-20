use std::str::FromStr;

use derive_more::derive::Display;
use winnow::ascii::{dec_uint, digit1};
use winnow::combinator::{alt, opt, preceded};
use winnow::error::{ContextError, StrContext};
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
    let digits = dec_uint::<_, u8, ContextError>;
    let (major, minor, patch) = seq!(
        digits.context(StrContext::Label("major")),
        _: '.',
        digits.context(StrContext::Label("minor")),
        opt(preceded('.', digits.context(StrContext::Label("patch"))))
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

fn pre_release_version(i: &mut &str) -> PResult<PreReleaseVersionNumber> {
    let (rv, pre_s, pre_n) = seq!(
        release_version,
        _: '-',
        alt(("pre", "rc")),
        digit1
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
    alt((
        release_version.map(VersionNumber::Release),
        take_while(1.., (AsChar::is_alphanum, '.', '-', '_', ' '))
            .map(|s: &str| VersionNumber::Other(s.into())),
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
    test_parse!(parse_release6: release_version("0.01.2") => panic);
    test_parse!(parse_release7: release_version("10.12.24") => ReleaseVersionNumber { major: 10, minor: 12, patch: 24 });
    test_parse!(parse_release8: release_version("1.0.256") => panic);
    test_bijective!(release_bijective: release_version("1.2.3"));

    test_parse!(parse_pre_release1: pre_release_version("1.2.3-pre1") => PreReleaseVersionNumber { release: ReleaseVersionNumber { major: 1, minor: 2, patch: 3 }, pre_release: _ });
    test_parse!(parse_pre_release2: pre_release_version("1.2.3-rc") => panic);
    test_parse!(parse_pre_release3: pre_release_version("1.2.3-prea") => panic);
    test_parse!(parse_pre_release4: pre_release_version("1.2-pre99") => PreReleaseVersionNumber { release: ReleaseVersionNumber { major: 1, minor: 2, patch: 0 }, pre_release: _ });
    test_bijective!(pre_release_bijective: pre_release_version("1.2.3-pre1"));

    test_parse!(parse_version1: version_number("1.2.3") => VersionNumber::Release(ReleaseVersionNumber { major: 1, minor: 2, patch: 3 }));
    test_parse!(parse_version2: version_number("") => panic);
    test_bijective!(version_bijective: version_number("foo"));
}
