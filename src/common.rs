use bincode::de::{BorrowDecoder, Decoder};
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{BorrowDecode, Decode, Encode};
use jiff::Timestamp;
use serde::Deserialize;

/// Wrapper type to implement bincode serialization for timestamps
#[derive(Debug, Deserialize, PartialEq, PartialOrd, Clone)]
#[serde(transparent)]
pub struct UtcDateTime(pub Timestamp);

impl Encode for UtcDateTime {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        let t = self.0;
        Encode::encode(&t.as_second(), encoder)?;
        Encode::encode(&t.subsec_nanosecond(), encoder)?;
        Ok(())
    }
}

impl<Context> Decode<Context> for UtcDateTime {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self(
            Timestamp::new(Decode::decode(decoder)?, Decode::decode(decoder)?)
                .map_err(|_| DecodeError::Other("invalid timestamp"))?,
        ))
    }
}

impl<'de, Context> BorrowDecode<'de, Context> for UtcDateTime {
    fn borrow_decode<D: BorrowDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        Ok(Self(
            Timestamp::new(
                BorrowDecode::borrow_decode(decoder)?,
                BorrowDecode::borrow_decode(decoder)?,
            )
            .map_err(|_| DecodeError::Other("invalid timestamp"))?,
        ))
    }
}
