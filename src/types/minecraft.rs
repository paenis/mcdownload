use std::str::FromStr;

use winnow::ascii::dec_uint;
use winnow::combinator::{alt, opt, preceded};
use winnow::error::{ContextError, StrContext};
use winnow::prelude::*;
use winnow::seq;
use winnow::stream::AsChar;
use winnow::token::take_while;

#[derive(Debug)]
struct ReleaseVersionNumber {
    major: u8,
    minor: u8,
    patch: u8,
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

#[derive(Debug)]
enum VersionNumber {
    Release(ReleaseVersionNumber),
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

    macro_rules! test {
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

    test!(release1: release_version("1.2.3") => ReleaseVersionNumber { major: 1, minor: 2, patch: 3 });
    test!(release2: release_version("1.2") => ReleaseVersionNumber { major: 1, minor: 2, patch: 0 });
    test!(release3: release_version("1") => panic);
    test!(release4: release_version("1.2.") => panic);
    test!(release5: release_version("1.2.3.") => panic);
    test!(release6: release_version("0.01.2") => panic);
    test!(release7: release_version("0.0.1") => ReleaseVersionNumber { major: 0, minor: 0, patch: 1 });
    test!(release9: release_version("10.12.24") => ReleaseVersionNumber { major: 10, minor: 12, patch: 24 });
    test!(release10: release_version("1.0.256") => panic);

    test!(version1: version_number("1.2.3") => VersionNumber::Release(ReleaseVersionNumber { major: 1, minor: 2, patch: 3 }));
    test!(version2: version_number("foo") => VersionNumber::Other(_));
    test!(version3: version_number("") => panic);
}
