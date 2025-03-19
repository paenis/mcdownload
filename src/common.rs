use bincode::de::{BorrowDecoder, Decoder};
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{BorrowDecode, Decode, Encode};
use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Wrapper type for `chrono::DateTime<Utc>`
///
/// This is needed because [`chrono::DateTime`] does not implement [`bincode::Encode`] or [`bincode::Decode`]
#[derive(Debug, Deserialize, PartialEq, PartialOrd, Clone)]
#[serde(transparent)]
pub struct UtcDateTime(pub DateTime<Utc>);

impl Encode for UtcDateTime {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        Encode::encode(&self.0.timestamp(), encoder)?;
        Encode::encode(&self.0.timestamp_subsec_nanos(), encoder)?;
        Ok(())
    }
}

impl<Context> Decode<Context> for UtcDateTime {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self(
            DateTime::from_timestamp(Decode::decode(decoder)?, Decode::decode(decoder)?)
                .ok_or(DecodeError::Other("invalid timestamp"))?,
        ))
    }
}

impl<'de, Context> BorrowDecode<'de, Context> for UtcDateTime {
    fn borrow_decode<D: BorrowDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        Ok(Self(
            DateTime::from_timestamp(
                BorrowDecode::borrow_decode(decoder)?,
                BorrowDecode::borrow_decode(decoder)?,
            )
            .ok_or(DecodeError::Other("invalid timestamp"))?,
        ))
    }
}
