use std::borrow::Cow;

use crate::{
    array::codec::{
        bytes_to_bytes::blosc::blosc_nbytes, BytesPartialDecoderTraits, CodecError, CodecOptions,
    },
    byte_range::ByteRange,
};

#[cfg(feature = "async")]
use crate::array::codec::AsyncBytesPartialDecoderTraits;

use super::{blosc_decompress_bytes_partial, blosc_typesize, blosc_validate};

/// Partial decoder for the `blosc` codec.
pub struct BloscPartialDecoder<'a> {
    input_handle: Box<dyn BytesPartialDecoderTraits + 'a>,
}

impl<'a> BloscPartialDecoder<'a> {
    pub fn new(input_handle: Box<dyn BytesPartialDecoderTraits + 'a>) -> Self {
        Self { input_handle }
    }
}

impl BytesPartialDecoderTraits for BloscPartialDecoder<'_> {
    fn partial_decode(
        &self,
        decoded_regions: &[ByteRange],
        options: &CodecOptions,
    ) -> Result<Option<Vec<Cow<'_, [u8]>>>, CodecError> {
        let encoded_value = self.input_handle.decode(options)?;
        let Some(encoded_value) = encoded_value else {
            return Ok(None);
        };

        if let Some(_destsize) = blosc_validate(&encoded_value) {
            let nbytes = blosc_nbytes(&encoded_value);
            let typesize = blosc_typesize(&encoded_value);
            if let (Some(nbytes), Some(typesize)) = (nbytes, typesize) {
                // FIXME: This needs coalescing to be efficient, for now CodecTraits::partial_decoder_decodes_all is set to true
                let mut decoded_byte_ranges = Vec::with_capacity(decoded_regions.len());
                for byte_range in decoded_regions {
                    let start = usize::try_from(byte_range.start(nbytes as u64)).unwrap();
                    let end = usize::try_from(byte_range.end(nbytes as u64)).unwrap();
                    decoded_byte_ranges.push(
                        blosc_decompress_bytes_partial(
                            &encoded_value,
                            start,
                            end - start,
                            typesize,
                        )
                        .map(Cow::Owned)
                        .map_err(|err| CodecError::from(err.to_string()))?,
                    );
                }
                return Ok(Some(decoded_byte_ranges));
            }
        }
        Err(CodecError::from("blosc encoded value is invalid"))
    }
}

#[cfg(feature = "async")]
/// Asynchronous partial decoder for the `blosc` codec.
pub struct AsyncBloscPartialDecoder<'a> {
    input_handle: Box<dyn AsyncBytesPartialDecoderTraits + 'a>,
}

#[cfg(feature = "async")]
impl<'a> AsyncBloscPartialDecoder<'a> {
    pub fn new(input_handle: Box<dyn AsyncBytesPartialDecoderTraits + 'a>) -> Self {
        Self { input_handle }
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl AsyncBytesPartialDecoderTraits for AsyncBloscPartialDecoder<'_> {
    async fn partial_decode(
        &self,
        decoded_regions: &[ByteRange],
        options: &CodecOptions,
    ) -> Result<Option<Vec<Cow<'_, [u8]>>>, CodecError> {
        let encoded_value = self.input_handle.decode(options).await?;
        let Some(encoded_value) = encoded_value else {
            return Ok(None);
        };

        if let Some(_destsize) = blosc_validate(&encoded_value) {
            let nbytes = blosc_nbytes(&encoded_value);
            let typesize = blosc_typesize(&encoded_value);
            if let (Some(nbytes), Some(typesize)) = (nbytes, typesize) {
                let mut decoded_byte_ranges = Vec::with_capacity(decoded_regions.len());
                for byte_range in decoded_regions {
                    let start = usize::try_from(byte_range.start(nbytes as u64)).unwrap();
                    let end = usize::try_from(byte_range.end(nbytes as u64)).unwrap();
                    decoded_byte_ranges.push(
                        blosc_decompress_bytes_partial(
                            &encoded_value,
                            start,
                            end - start,
                            typesize,
                        )
                        .map(Cow::Owned)
                        .map_err(|err| CodecError::from(err.to_string()))?,
                    );
                }
                return Ok(Some(decoded_byte_ranges));
            }
        }
        Err(CodecError::from("blosc encoded value is invalid"))
    }
}
