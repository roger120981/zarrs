use std::sync::Arc;

use futures::StreamExt;

use crate::{
    array_subset::ArraySubset,
    node::NodePath,
    storage::{data_key, meta_key, AsyncReadableStorageTraits, StorageHandle},
};

use super::{
    codec::{
        options::CodecOptions, ArrayCodecTraits, ArrayToBytesCodecTraits,
        AsyncArrayPartialDecoderTraits, AsyncStoragePartialDecoder, CodecError,
    },
    concurrency::concurrency_chunks_and_codec,
    transmute_from_bytes_vec,
    unsafe_cell_slice::UnsafeCellSlice,
    validate_element_size, Array, ArrayCreateError, ArrayError, ArrayMetadata, ArrayView,
};

#[cfg(feature = "ndarray")]
use super::elements_to_ndarray;

impl<TStorage: ?Sized + AsyncReadableStorageTraits + 'static> Array<TStorage> {
    /// Async variant of [`new`](Array::new).
    #[allow(clippy::missing_errors_doc)]
    pub async fn async_new(storage: Arc<TStorage>, path: &str) -> Result<Self, ArrayCreateError> {
        let node_path = NodePath::new(path)?;
        let key = meta_key(&node_path);
        let metadata: ArrayMetadata = serde_json::from_slice(
            &storage
                .get(&key)
                .await?
                .ok_or(ArrayCreateError::MissingMetadata)?,
        )
        .map_err(|err| crate::storage::StorageError::InvalidMetadata(key, err.to_string()))?;
        Self::new_with_metadata(storage, path, metadata)
    }

    /// Async variant of [`retrieve_chunk_if_exists`](Array::retrieve_chunk_if_exists).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_chunk_if_exists(
        &self,
        chunk_indices: &[u64],
    ) -> Result<Option<Vec<u8>>, ArrayError> {
        self.async_retrieve_chunk_if_exists_opt(chunk_indices, &CodecOptions::default())
            .await
    }

    /// Async variant of [`retrieve_chunk_elements_if_exists`](Array::retrieve_chunk_elements_if_exists).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_chunk_elements_if_exists<T: bytemuck::Pod + Send + Sync>(
        &self,
        chunk_indices: &[u64],
    ) -> Result<Option<Vec<T>>, ArrayError> {
        self.async_retrieve_chunk_elements_if_exists_opt(chunk_indices, &CodecOptions::default())
            .await
    }

    #[cfg(feature = "ndarray")]
    /// Async variant of [`retrieve_chunk_ndarray_if_exists`](Array::retrieve_chunk_ndarray_if_exists).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_chunk_ndarray_if_exists<T: bytemuck::Pod + Send + Sync>(
        &self,
        chunk_indices: &[u64],
    ) -> Result<Option<ndarray::ArrayD<T>>, ArrayError> {
        self.async_retrieve_chunk_ndarray_if_exists_opt(chunk_indices, &CodecOptions::default())
            .await
    }

    /// Async variant of [`retrieve_chunk`](Array::retrieve_chunk).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_chunk(&self, chunk_indices: &[u64]) -> Result<Vec<u8>, ArrayError> {
        self.async_retrieve_chunk_opt(chunk_indices, &CodecOptions::default())
            .await
    }

    /// Async variant of [`retrieve_chunk_elements`](Array::retrieve_chunk_elements).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_chunk_elements<T: bytemuck::Pod + Send + Sync>(
        &self,
        chunk_indices: &[u64],
    ) -> Result<Vec<T>, ArrayError> {
        self.async_retrieve_chunk_elements_opt(chunk_indices, &CodecOptions::default())
            .await
    }

    #[cfg(feature = "ndarray")]
    /// Async variant of [`retrieve_chunk_ndarray`](Array::retrieve_chunk_ndarray).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_chunk_ndarray<T: bytemuck::Pod + Send + Sync>(
        &self,
        chunk_indices: &[u64],
    ) -> Result<ndarray::ArrayD<T>, ArrayError> {
        self.async_retrieve_chunk_ndarray_opt(chunk_indices, &CodecOptions::default())
            .await
    }

    /// Async variant of [`retrieve_chunk_into_array_view`](Array::retrieve_chunk_into_array_view).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_chunk_into_array_view(
        &self,
        chunk_indices: &[u64],
        array_view: &ArrayView<'_>,
    ) -> Result<(), ArrayError> {
        self.async_retrieve_chunk_into_array_view_opt(
            chunk_indices,
            array_view,
            &CodecOptions::default(),
        )
        .await
    }

    /// Async variant of [`retrieve_chunks`](Array::retrieve_chunks).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_chunks(&self, chunks: &ArraySubset) -> Result<Vec<u8>, ArrayError> {
        self.async_retrieve_chunks_opt(chunks, &CodecOptions::default())
            .await
    }

    /// Async variant of [`retrieve_chunks_elements`](Array::retrieve_chunks_elements).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_chunks_elements<T: bytemuck::Pod + Send + Sync>(
        &self,
        chunks: &ArraySubset,
    ) -> Result<Vec<T>, ArrayError> {
        self.async_retrieve_chunks_elements_opt(chunks, &CodecOptions::default())
            .await
    }

    #[cfg(feature = "ndarray")]
    /// Async variant of [`retrieve_chunks_ndarray`](Array::retrieve_chunks_ndarray).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_chunks_ndarray<T: bytemuck::Pod + Send + Sync>(
        &self,
        chunks: &ArraySubset,
    ) -> Result<ndarray::ArrayD<T>, ArrayError> {
        self.async_retrieve_chunks_ndarray_opt(chunks, &CodecOptions::default())
            .await
    }

    /// Async variant of [`retrieve_chunks_into_array_view`](Array::retrieve_chunks_into_array_view).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_chunks_into_array_view(
        &self,
        chunks: &ArraySubset,
        array_view: &ArrayView<'_>,
    ) -> Result<(), ArrayError> {
        self.async_retrieve_chunks_into_array_view_opt(chunks, array_view, &CodecOptions::default())
            .await
    }

    /// Async variant of [`retrieve_chunk_subset`](Array::retrieve_chunk_subset).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_chunk_subset(
        &self,
        chunk_indices: &[u64],
        chunk_subset: &ArraySubset,
    ) -> Result<Vec<u8>, ArrayError> {
        self.async_retrieve_chunk_subset_opt(chunk_indices, chunk_subset, &CodecOptions::default())
            .await
    }

    /// Async variant of [`retrieve_chunk_subset_elements`](Array::retrieve_chunk_subset_elements).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_chunk_subset_elements<T: bytemuck::Pod + Send + Sync>(
        &self,
        chunk_indices: &[u64],
        chunk_subset: &ArraySubset,
    ) -> Result<Vec<T>, ArrayError> {
        self.async_retrieve_chunk_subset_elements_opt(
            chunk_indices,
            chunk_subset,
            &CodecOptions::default(),
        )
        .await
    }

    #[cfg(feature = "ndarray")]
    /// Async variant of [`retrieve_chunk_subset_ndarray`](Array::retrieve_chunk_subset_ndarray).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_chunk_subset_ndarray<T: bytemuck::Pod + Send + Sync>(
        &self,
        chunk_indices: &[u64],
        chunk_subset: &ArraySubset,
    ) -> Result<ndarray::ArrayD<T>, ArrayError> {
        self.async_retrieve_chunk_subset_ndarray_opt(
            chunk_indices,
            chunk_subset,
            &CodecOptions::default(),
        )
        .await
    }

    /// Async variant of [`retrieve_chunk_subset_into_array_view`](Array::retrieve_chunk_subset_into_array_view).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_chunk_subset_into_array_view(
        &self,
        chunk_indices: &[u64],
        chunk_subset: &ArraySubset,
        array_view: &ArrayView<'_>,
    ) -> Result<(), ArrayError> {
        self.async_retrieve_chunk_subset_into_array_view_opt(
            chunk_indices,
            chunk_subset,
            array_view,
            &CodecOptions::default(),
        )
        .await
    }

    /// Async variant of [`retrieve_array_subset`](Array::retrieve_array_subset).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_array_subset(
        &self,
        array_subset: &ArraySubset,
    ) -> Result<Vec<u8>, ArrayError> {
        self.async_retrieve_array_subset_opt(array_subset, &CodecOptions::default())
            .await
    }

    /// Async variant of [`retrieve_array_subset_elements`](Array::retrieve_array_subset_elements).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_array_subset_elements<T: bytemuck::Pod + Send + Sync>(
        &self,
        array_subset: &ArraySubset,
    ) -> Result<Vec<T>, ArrayError> {
        self.async_retrieve_array_subset_elements_opt(array_subset, &CodecOptions::default())
            .await
    }

    #[cfg(feature = "ndarray")]
    /// Async variant of [`retrieve_array_subset_ndarray`](Array::retrieve_array_subset_ndarray).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_array_subset_ndarray<T: bytemuck::Pod + Send + Sync>(
        &self,
        array_subset: &ArraySubset,
    ) -> Result<ndarray::ArrayD<T>, ArrayError> {
        self.async_retrieve_array_subset_ndarray_opt(array_subset, &CodecOptions::default())
            .await
    }

    /// Async variant of [`retrieve_array_subset_into_array_view`](Array::retrieve_array_subset_into_array_view).
    #[allow(clippy::missing_errors_doc)]
    pub async fn async_retrieve_array_subset_into_array_view(
        &self,
        array_subset: &ArraySubset,
        array_view: &ArrayView<'_>,
    ) -> Result<(), ArrayError> {
        self.async_retrieve_array_subset_into_array_view_opt(
            array_subset,
            array_view,
            &CodecOptions::default(),
        )
        .await
    }

    /// Async variant of [`partial_decoder`](Array::partial_decoder).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_partial_decoder<'a>(
        &'a self,
        chunk_indices: &[u64],
    ) -> Result<Box<dyn AsyncArrayPartialDecoderTraits + 'a>, ArrayError> {
        self.async_partial_decoder_opt(chunk_indices, &CodecOptions::default())
            .await
    }

    /////////////////////////////////////////////////////////////////////////////
    // Advanced methods
    /////////////////////////////////////////////////////////////////////////////

    /// Async variant of [`retrieve_chunk_if_exists_opt`](Array::retrieve_chunk_if_exists_opt).
    #[allow(clippy::missing_errors_doc)]
    pub async fn async_retrieve_chunk_if_exists_opt(
        &self,
        chunk_indices: &[u64],
        options: &CodecOptions,
    ) -> Result<Option<Vec<u8>>, ArrayError> {
        if chunk_indices.len() != self.dimensionality() {
            return Err(ArrayError::InvalidChunkGridIndicesError(
                chunk_indices.to_vec(),
            ));
        }
        let storage_handle = Arc::new(StorageHandle::new(self.storage.clone()));
        let storage_transformer = self
            .storage_transformers()
            .create_async_readable_transformer(storage_handle);
        let chunk_encoded = crate::storage::async_retrieve_chunk(
            &*storage_transformer,
            self.path(),
            chunk_indices,
            self.chunk_key_encoding(),
        )
        .await
        .map_err(ArrayError::StorageError)?;
        if let Some(chunk_encoded) = chunk_encoded {
            let chunk_representation = self.chunk_array_representation(chunk_indices)?;
            let chunk_decoded = self
                .codecs()
                .decode(chunk_encoded, &chunk_representation, options)
                .map_err(ArrayError::CodecError)?;
            let chunk_decoded_size =
                chunk_representation.num_elements_usize() * chunk_representation.data_type().size();
            if chunk_decoded.len() == chunk_decoded_size {
                Ok(Some(chunk_decoded))
            } else {
                Err(ArrayError::UnexpectedChunkDecodedSize(
                    chunk_decoded.len(),
                    chunk_decoded_size,
                ))
            }
        } else {
            Ok(None)
        }
    }

    /// Async variant of [`retrieve_chunk_opt`](Array::retrieve_chunk_opt).
    #[allow(clippy::missing_errors_doc)]
    pub async fn async_retrieve_chunk_opt(
        &self,
        chunk_indices: &[u64],
        options: &CodecOptions,
    ) -> Result<Vec<u8>, ArrayError> {
        let chunk = self
            .async_retrieve_chunk_if_exists_opt(chunk_indices, options)
            .await?;
        if let Some(chunk) = chunk {
            Ok(chunk)
        } else {
            let chunk_representation = self.chunk_array_representation(chunk_indices)?;
            let fill_value = chunk_representation.fill_value().as_ne_bytes();
            Ok(fill_value.repeat(chunk_representation.num_elements_usize()))
        }
    }

    /// Async variant of [`retrieve_chunk_elements_if_exists_opt`](Array::retrieve_chunk_elements_if_exists_opt).
    #[allow(clippy::missing_errors_doc)]
    pub async fn async_retrieve_chunk_elements_if_exists_opt<T: bytemuck::Pod + Send + Sync>(
        &self,
        chunk_indices: &[u64],
        options: &CodecOptions,
    ) -> Result<Option<Vec<T>>, ArrayError> {
        validate_element_size::<T>(self.data_type())?;
        let bytes = self
            .async_retrieve_chunk_if_exists_opt(chunk_indices, options)
            .await?;
        Ok(bytes.map(|bytes| transmute_from_bytes_vec::<T>(bytes)))
    }

    /// Async variant of [`retrieve_chunk_elements_opt`](Array::retrieve_chunk_elements_opt).
    #[allow(clippy::missing_errors_doc)]
    pub async fn async_retrieve_chunk_elements_opt<T: bytemuck::Pod + Send + Sync>(
        &self,
        chunk_indices: &[u64],
        options: &CodecOptions,
    ) -> Result<Vec<T>, ArrayError> {
        validate_element_size::<T>(self.data_type())?;
        let bytes = self
            .async_retrieve_chunk_opt(chunk_indices, options)
            .await?;
        Ok(transmute_from_bytes_vec::<T>(bytes))
    }

    #[cfg(feature = "ndarray")]
    /// Async variant of [`retrieve_chunk_ndarray_if_exists_opt`](Array::retrieve_chunk_ndarray_if_exists_opt).
    #[allow(clippy::missing_errors_doc)]
    pub async fn async_retrieve_chunk_ndarray_if_exists_opt<T: bytemuck::Pod + Send + Sync>(
        &self,
        chunk_indices: &[u64],
        options: &CodecOptions,
    ) -> Result<Option<ndarray::ArrayD<T>>, ArrayError> {
        // validate_element_size::<T>(self.data_type())?; in // async_retrieve_chunk_elements_if_exists
        let shape = self
            .chunk_grid()
            .chunk_shape_u64(chunk_indices, self.shape())?
            .ok_or_else(|| ArrayError::InvalidChunkGridIndicesError(chunk_indices.to_vec()))?;
        let elements = self
            .async_retrieve_chunk_elements_if_exists_opt(chunk_indices, options)
            .await?;
        if let Some(elements) = elements {
            Ok(Some(elements_to_ndarray(&shape, elements)?))
        } else {
            Ok(None)
        }
    }

    #[cfg(feature = "ndarray")]
    /// Async variant of [`retrieve_chunk_ndarray_opt`](Array::retrieve_chunk_ndarray_opt).
    #[allow(clippy::missing_errors_doc)]
    pub async fn async_retrieve_chunk_ndarray_opt<T: bytemuck::Pod + Send + Sync>(
        &self,
        chunk_indices: &[u64],
        options: &CodecOptions,
    ) -> Result<ndarray::ArrayD<T>, ArrayError> {
        // validate_element_size::<T>(self.data_type())?; // in async_retrieve_chunk_elements
        let shape = self
            .chunk_grid()
            .chunk_shape_u64(chunk_indices, self.shape())?
            .ok_or_else(|| ArrayError::InvalidChunkGridIndicesError(chunk_indices.to_vec()))?;
        let elements = self
            .async_retrieve_chunk_elements_opt(chunk_indices, options)
            .await?;
        elements_to_ndarray(&shape, elements)
    }

    /// Async variant of [`retrieve_chunk_into_array_view_opt`](Array::retrieve_chunk_into_array_view_opt).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_chunk_into_array_view_opt(
        &self,
        chunk_indices: &[u64],
        array_view: &ArrayView<'_>,
        options: &CodecOptions,
    ) -> Result<(), ArrayError> {
        let chunk_representation = self.chunk_array_representation(chunk_indices)?;
        let chunk_shape_u64 = chunk_representation.shape_u64();
        if chunk_shape_u64 != array_view.subset().shape() {
            return Err(ArrayError::InvalidArraySubset(
                array_view.subset().clone(),
                chunk_shape_u64,
            ));
        }

        let storage_handle = Arc::new(StorageHandle::new(self.storage.clone()));
        let storage_transformer = self
            .storage_transformers()
            .create_async_readable_transformer(storage_handle);
        let chunk_encoded = crate::storage::async_retrieve_chunk(
            &*storage_transformer,
            self.path(),
            chunk_indices,
            self.chunk_key_encoding(),
        )
        .await
        .map_err(ArrayError::StorageError)?;
        if let Some(chunk_encoded) = chunk_encoded {
            self.codecs()
                .decode_into_array_view(&chunk_encoded, &chunk_representation, array_view, options)
                .map_err(ArrayError::CodecError)
        } else {
            // fill array_view with fill value
            let contiguous_indices = unsafe {
                array_view
                    .subset()
                    .contiguous_linearised_indices_unchecked(array_view.array_shape())
            };
            let element_size = chunk_representation.element_size();
            let length = contiguous_indices.contiguous_elements_usize() * element_size;
            let fill = self
                .fill_value()
                .as_ne_bytes()
                .repeat(contiguous_indices.contiguous_elements_usize());
            // FIXME: Par iteration?
            let output = unsafe { array_view.bytes_mut() };
            for (array_subset_element_index, _num_elements) in &contiguous_indices {
                let output_offset =
                    usize::try_from(array_subset_element_index).unwrap() * element_size;
                debug_assert!((output_offset + length) <= output.len());
                output[output_offset..output_offset + length].copy_from_slice(&fill);
            }
            Ok(())
        }
    }

    /// Async variant of [`retrieve_chunks_opt`](Array::retrieve_chunks_opt).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_chunks_opt(
        &self,
        chunks: &ArraySubset,
        options: &CodecOptions,
    ) -> Result<Vec<u8>, ArrayError> {
        if chunks.dimensionality() != self.dimensionality() {
            return Err(ArrayError::InvalidArraySubset(
                chunks.clone(),
                self.shape().to_vec(),
            ));
        }

        let array_subset = Arc::new(self.chunks_subset(chunks)?);

        // Retrieve chunk bytes
        let num_chunks = chunks.num_elements_usize();
        match num_chunks {
            0 => Ok(vec![]),
            1 => {
                let chunk_indices = chunks.start();
                self.async_retrieve_chunk_opt(chunk_indices, options).await
            }
            _ => {
                // Decode chunks and copy to output
                let size_output =
                    usize::try_from(array_subset.num_elements() * self.data_type().size() as u64)
                        .unwrap();

                // Calculate chunk/codec concurrency
                let chunk_representation =
                    self.chunk_array_representation(&vec![0; self.dimensionality()])?;
                let codec_concurrency =
                    self.recommended_codec_concurrency(&chunk_representation)?;
                let (chunk_concurrent_limit, options) = concurrency_chunks_and_codec(
                    options.concurrent_target(),
                    num_chunks,
                    options,
                    &codec_concurrency,
                );

                let mut output = Vec::with_capacity(size_output);
                {
                    let output_slice =
                        UnsafeCellSlice::new_from_vec_with_spare_capacity(&mut output);
                    let indices = chunks.indices();
                    let chunk0_subset = self.chunk_subset(chunks.start())?;
                    let futures = indices.into_iter().map(|chunk_indices| {
                        let options = options.clone();
                        let array_subset = array_subset.clone();
                        let chunk_subset = self.chunk_subset(&chunk_indices).unwrap(); // FIXME: unwrap
                        let array_view_subset =
                            unsafe { chunk_subset.relative_to_unchecked(chunk0_subset.start()) };
                        async move {
                            self.async_retrieve_chunk_into_array_view_opt(
                                &chunk_indices,
                                &ArrayView::new(
                                    unsafe { output_slice.get() },
                                    array_subset.shape(),
                                    array_view_subset,
                                )
                                .unwrap(), // FIXME: unwrap
                                &options,
                            )
                            .await
                        }
                    });
                    let mut stream =
                        futures::stream::iter(futures).buffer_unordered(chunk_concurrent_limit);
                    while let Some(item) = stream.next().await {
                        item?;
                    }
                }
                unsafe { output.set_len(size_output) };
                Ok(output)
            }
        }
    }

    /// Async variant of [`retrieve_chunks_elements_opt`](Array::retrieve_chunks_elements_opt).
    #[allow(clippy::missing_errors_doc)]
    pub async fn async_retrieve_chunks_elements_opt<T: bytemuck::Pod + Send + Sync>(
        &self,
        chunks: &ArraySubset,
        options: &CodecOptions,
    ) -> Result<Vec<T>, ArrayError> {
        validate_element_size::<T>(self.data_type())?;
        let bytes = self.async_retrieve_chunks_opt(chunks, options).await?;
        Ok(transmute_from_bytes_vec::<T>(bytes))
    }

    #[cfg(feature = "ndarray")]
    /// Async variant of [`retrieve_chunks_ndarray_opt`](Array::retrieve_chunks_ndarray_opt).
    #[allow(clippy::missing_errors_doc)]
    pub async fn async_retrieve_chunks_ndarray_opt<T: bytemuck::Pod + Send + Sync>(
        &self,
        chunks: &ArraySubset,
        options: &CodecOptions,
    ) -> Result<ndarray::ArrayD<T>, ArrayError> {
        validate_element_size::<T>(self.data_type())?;
        let array_subset = self.chunks_subset(chunks)?;
        let elements = self
            .async_retrieve_chunks_elements_opt(chunks, options)
            .await?;
        elements_to_ndarray(array_subset.shape(), elements)
    }

    /// Async variant of [`retrieve_array_subset_opt`](Array::retrieve_array_subset_opt).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    #[allow(clippy::too_many_lines)]
    pub async fn async_retrieve_array_subset_opt(
        &self,
        array_subset: &ArraySubset,
        options: &CodecOptions,
    ) -> Result<Vec<u8>, ArrayError> {
        if array_subset.dimensionality() != self.dimensionality() {
            return Err(ArrayError::InvalidArraySubset(
                array_subset.clone(),
                self.shape().to_vec(),
            ));
        }

        // Find the chunks intersecting this array subset
        let chunks = self.chunks_in_array_subset(array_subset)?;
        let Some(chunks) = chunks else {
            return Err(ArrayError::InvalidArraySubset(
                array_subset.clone(),
                self.shape().to_vec(),
            ));
        };

        // Retrieve chunk bytes
        let num_chunks = chunks.num_elements_usize();
        match num_chunks {
            0 => Ok(self
                .fill_value()
                .as_ne_bytes()
                .repeat(array_subset.num_elements_usize())),
            1 => {
                let chunk_indices = chunks.start();
                let chunk_subset = self.chunk_subset(chunk_indices)?;
                if &chunk_subset == array_subset {
                    // Single chunk fast path if the array subset domain matches the chunk domain
                    self.async_retrieve_chunk_opt(chunk_indices, options).await
                } else {
                    let array_subset_in_chunk_subset =
                        unsafe { array_subset.relative_to_unchecked(chunk_subset.start()) };
                    self.async_retrieve_chunk_subset_opt(
                        chunk_indices,
                        &array_subset_in_chunk_subset,
                        options,
                    )
                    .await
                }
            }
            _ => {
                // Decode chunks and copy to output
                let size_output =
                    usize::try_from(array_subset.num_elements() * self.data_type().size() as u64)
                        .unwrap();

                // Calculate chunk/codec concurrency
                let chunk_representation =
                    self.chunk_array_representation(&vec![0; self.dimensionality()])?;
                let codec_concurrency =
                    self.recommended_codec_concurrency(&chunk_representation)?;
                let (chunk_concurrent_limit, options) = concurrency_chunks_and_codec(
                    options.concurrent_target(),
                    num_chunks,
                    options,
                    &codec_concurrency,
                );

                // let mut output = vec![0; size_output];
                // let output_slice = output.as_mut_slice();
                let mut output = Vec::with_capacity(size_output);
                {
                    let output = UnsafeCellSlice::new_from_vec_with_spare_capacity(&mut output);
                    let retrieve_chunk = |chunk_indices: Vec<u64>| {
                        let options = options.clone();
                        let chunk_subset = self.chunk_subset(&chunk_indices).unwrap(); // FIXME: unwrap
                        let chunk_subset_in_array_subset =
                            unsafe { chunk_subset.overlap_unchecked(array_subset) };
                        let chunk_subset = unsafe {
                            chunk_subset_in_array_subset.relative_to_unchecked(chunk_subset.start())
                        };
                        let array_view_subset = unsafe {
                            chunk_subset_in_array_subset.relative_to_unchecked(array_subset.start())
                        };
                        let array_view = ArrayView::new(
                            unsafe { output.get() },
                            array_subset.shape(),
                            array_view_subset,
                        )
                        .unwrap(); // FIXME: unwrap
                        async move {
                            self.async_retrieve_chunk_subset_into_array_view_opt(
                                &chunk_indices,
                                &chunk_subset,
                                &array_view,
                                &options,
                            )
                            .await
                        }
                    };
                    let indices = chunks.indices();
                    let futures = indices.into_iter().map(retrieve_chunk);
                    let mut stream =
                        futures::stream::iter(futures).buffer_unordered(chunk_concurrent_limit);
                    while let Some(item) = stream.next().await {
                        item?;
                    }
                }
                unsafe { output.set_len(size_output) };
                Ok(output)
            }
        }
    }

    /// Async variant of [`retrieve_array_subset_elements_opt`](Array::retrieve_array_subset_elements_opt).
    #[allow(clippy::missing_errors_doc)]
    pub async fn async_retrieve_array_subset_elements_opt<T: bytemuck::Pod + Send + Sync>(
        &self,
        array_subset: &ArraySubset,
        options: &CodecOptions,
    ) -> Result<Vec<T>, ArrayError> {
        validate_element_size::<T>(self.data_type())?;
        let bytes = self
            .async_retrieve_array_subset_opt(array_subset, options)
            .await?;
        Ok(transmute_from_bytes_vec::<T>(bytes))
    }

    #[cfg(feature = "ndarray")]
    /// Async variant of [`retrieve_array_subset_ndarray_opt`](Array::retrieve_array_subset_ndarray_opt).
    #[allow(clippy::missing_errors_doc)]
    pub async fn async_retrieve_array_subset_ndarray_opt<T: bytemuck::Pod + Send + Sync>(
        &self,
        array_subset: &ArraySubset,
        options: &CodecOptions,
    ) -> Result<ndarray::ArrayD<T>, ArrayError> {
        // validate_element_size::<T>(self.data_type())?; // in async_retrieve_array_subset_elements
        let elements = self
            .async_retrieve_array_subset_elements_opt(array_subset, options)
            .await?;
        elements_to_ndarray(array_subset.shape(), elements)
    }

    /// Async variant of [`retrieve_chunks_into_array_view_opt`](Array::retrieve_chunks_into_array_view_opt).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_chunks_into_array_view_opt(
        &self,
        chunks: &ArraySubset,
        array_view: &ArrayView<'_>,
        options: &CodecOptions,
    ) -> Result<(), ArrayError> {
        if chunks.dimensionality() != self.dimensionality() {
            return Err(ArrayError::InvalidArraySubset(
                chunks.clone(),
                self.chunk_grid_shape()
                    .unwrap_or_else(|| vec![0u64; self.dimensionality()]),
            ));
        }
        let num_chunks = chunks.num_elements_usize();
        if num_chunks == 0 {
            return Ok(());
        }

        let array_subset = self.chunks_subset(chunks)?;
        if array_subset.shape() != array_view.subset().shape() {
            return Err(ArrayError::InvalidArraySubset(
                array_subset.clone(),
                array_view.subset().shape().to_vec(),
            ));
        }

        if num_chunks == 1 {
            let chunk_indices = chunks.start();
            self.async_retrieve_chunk_into_array_view_opt(chunk_indices, array_view, options)
                .await
        } else {
            // Calculate chunk/codec concurrency
            let chunk_representation =
                self.chunk_array_representation(&vec![0; self.dimensionality()])?;
            let codec_concurrency = self.recommended_codec_concurrency(&chunk_representation)?;
            let (chunk_concurrent_limit, options) = concurrency_chunks_and_codec(
                options.concurrent_target(),
                num_chunks,
                options,
                &codec_concurrency,
            );

            {
                let indices = chunks.indices();
                let futures = indices.into_iter().map(|chunk_indices| {
                    let chunk0_start = array_subset.start().to_vec();
                    let options = options.clone();
                    async move {
                        let chunk_subset = self.chunk_subset(&chunk_indices).unwrap();
                        let array_view_subset =
                            unsafe { chunk_subset.relative_to_unchecked(&chunk0_start) };
                        self.async_retrieve_chunk_into_array_view_opt(
                            &chunk_indices,
                            &unsafe { array_view.subset_view(&array_view_subset) }.unwrap(), // FIXME: unwrap
                            &options,
                        )
                        .await
                    }
                });
                let mut stream =
                    futures::stream::iter(futures).buffer_unordered(chunk_concurrent_limit);
                while let Some(item) = stream.next().await {
                    item?;
                }
            }
            Ok(())
        }
    }

    /// Async variant of [`retrieve_array_subset_into_array_view_opt`](Array::retrieve_array_subset_into_array_view_opt).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_array_subset_into_array_view_opt(
        &self,
        array_subset: &ArraySubset,
        array_view: &ArrayView<'_>,
        options: &CodecOptions,
    ) -> Result<(), ArrayError> {
        if array_subset.shape() != array_view.subset().shape() {
            return Err(ArrayError::InvalidArraySubset(
                array_subset.clone(),
                array_view.subset().shape().to_vec(),
            ));
        }

        // Find the chunks intersecting this array subset
        let chunks = self.chunks_in_array_subset(array_subset)?;
        let Some(chunks) = chunks else {
            return Err(ArrayError::InvalidArraySubset(
                array_subset.clone(),
                self.shape().to_vec(),
            ));
        };

        // Retrieve chunk bytes
        let num_chunks = chunks.num_elements_usize();
        match num_chunks {
            0 => Ok(()),
            1 => {
                let chunk_indices = chunks.start();
                let chunk_subset = self.chunk_subset(chunk_indices).unwrap();
                if &chunk_subset == array_subset {
                    // Single chunk fast path if the array subset domain matches the chunk domain
                    let array_view_subset =
                        unsafe { chunk_subset.relative_to_unchecked(array_subset.start()) };
                    self.async_retrieve_chunk_into_array_view_opt(
                        chunk_indices,
                        &unsafe { array_view.subset_view(&array_view_subset) }
                            .map_err(|err| CodecError::from(err.to_string()))?,
                        options,
                    )
                    .await
                } else {
                    let chunk_subset_in_array_subset =
                        unsafe { chunk_subset.overlap_unchecked(array_subset) };
                    let chunk_subset = unsafe {
                        chunk_subset_in_array_subset.relative_to_unchecked(chunk_subset.start())
                    };
                    let array_view_subset = unsafe {
                        chunk_subset_in_array_subset.relative_to_unchecked(array_subset.start())
                    };
                    self.async_retrieve_chunk_subset_into_array_view_opt(
                        chunk_indices,
                        &chunk_subset,
                        &unsafe { array_view.subset_view(&array_view_subset) }
                            .map_err(|err| CodecError::from(err.to_string()))?,
                        options,
                    )
                    .await
                }
            }
            _ => {
                // Calculate chunk/codec concurrency
                let chunk_representation =
                    self.chunk_array_representation(&vec![0; self.dimensionality()])?;
                let codec_concurrency =
                    self.recommended_codec_concurrency(&chunk_representation)?;
                let (chunk_concurrent_limit, options) = concurrency_chunks_and_codec(
                    options.concurrent_target(),
                    num_chunks,
                    options,
                    &codec_concurrency,
                );

                {
                    let indices = chunks.indices();
                    let futures = indices.into_iter().map(|chunk_indices| {
                        let chunk_subset = self.chunk_subset(&chunk_indices).unwrap();
                        let chunk_subset_in_array_subset =
                            unsafe { chunk_subset.overlap_unchecked(array_subset) };
                        let chunk_subset = unsafe {
                            chunk_subset_in_array_subset.relative_to_unchecked(chunk_subset.start())
                        };
                        let array_view_subset = unsafe {
                            chunk_subset_in_array_subset.relative_to_unchecked(array_subset.start())
                        };
                        let options = options.clone();
                        async move {
                            self.async_retrieve_chunk_subset_into_array_view_opt(
                                &chunk_indices,
                                &chunk_subset,
                                &unsafe { array_view.subset_view(&array_view_subset) }.unwrap(),
                                &options,
                            )
                            .await
                        }
                    });
                    let mut stream =
                        futures::stream::iter(futures).buffer_unordered(chunk_concurrent_limit);
                    while let Some(item) = stream.next().await {
                        item?;
                    }
                }
                Ok(())
            }
        }
    }

    /// Async variant of [`retrieve_chunk_subset_opt`](Array::retrieve_chunk_subset_opt).
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn async_retrieve_chunk_subset_opt(
        &self,
        chunk_indices: &[u64],
        chunk_subset: &ArraySubset,
        options: &CodecOptions,
    ) -> Result<Vec<u8>, ArrayError> {
        let chunk_representation = self.chunk_array_representation(chunk_indices)?;
        if !chunk_subset.inbounds(&chunk_representation.shape_u64()) {
            return Err(ArrayError::InvalidArraySubset(
                chunk_subset.clone(),
                self.shape().to_vec(),
            ));
        }

        let storage_handle = Arc::new(StorageHandle::new(self.storage.clone()));
        let storage_transformer = self
            .storage_transformers()
            .create_async_readable_transformer(storage_handle);
        let input_handle = Box::new(AsyncStoragePartialDecoder::new(
            storage_transformer,
            data_key(self.path(), chunk_indices, self.chunk_key_encoding()),
        ));

        let decoded_bytes = self
            .codecs()
            .async_partial_decoder(input_handle, &chunk_representation, options)
            .await?
            .partial_decode_opt(&[chunk_subset.clone()], options)
            .await?
            .pop()
            .unwrap();

        let expected_size = chunk_subset.num_elements_usize() * self.data_type().size();
        if decoded_bytes.len() == chunk_subset.num_elements_usize() * self.data_type().size() {
            Ok(decoded_bytes)
        } else {
            Err(ArrayError::UnexpectedChunkDecodedSize(
                decoded_bytes.len(),
                expected_size,
            ))
        }
    }

    /// Async variant of [`retrieve_chunk_subset_elements_opt`](Array::retrieve_chunk_subset_elements_opt).
    #[allow(clippy::missing_errors_doc)]
    pub async fn async_retrieve_chunk_subset_elements_opt<T: bytemuck::Pod + Send + Sync>(
        &self,
        chunk_indices: &[u64],
        chunk_subset: &ArraySubset,
        options: &CodecOptions,
    ) -> Result<Vec<T>, ArrayError> {
        validate_element_size::<T>(self.data_type())?;
        let bytes = self
            .async_retrieve_chunk_subset_opt(chunk_indices, chunk_subset, options)
            .await?;
        Ok(transmute_from_bytes_vec::<T>(bytes))
    }

    #[cfg(feature = "ndarray")]
    /// Async variant of [`retrieve_chunk_subset_ndarray_opt`](Array::retrieve_chunk_subset_ndarray_opt).
    #[allow(clippy::missing_errors_doc)]
    pub async fn async_retrieve_chunk_subset_ndarray_opt<T: bytemuck::Pod + Send + Sync>(
        &self,
        chunk_indices: &[u64],
        chunk_subset: &ArraySubset,
        options: &CodecOptions,
    ) -> Result<ndarray::ArrayD<T>, ArrayError> {
        // validate_element_size::<T>(self.data_type())?; // in async_retrieve_chunk_subset_elements
        let elements = self
            .async_retrieve_chunk_subset_elements_opt(chunk_indices, chunk_subset, options)
            .await?;
        elements_to_ndarray(chunk_subset.shape(), elements)
    }

    /// Async variant of [`retrieve_chunk_subset_into_array_view_opt`](Array::retrieve_chunk_subset_into_array_view_opt).
    #[allow(clippy::missing_errors_doc)]
    pub async fn async_retrieve_chunk_subset_into_array_view_opt(
        &self,
        chunk_indices: &[u64],
        chunk_subset: &ArraySubset,
        array_view: &ArrayView<'_>,
        options: &CodecOptions,
    ) -> Result<(), ArrayError> {
        if chunk_subset.shape() != array_view.subset().shape() {
            return Err(ArrayError::InvalidArraySubset(
                chunk_subset.clone(),
                array_view.subset().shape().to_vec(),
            ));
        }

        let chunk_representation = self.chunk_array_representation(chunk_indices)?;
        if chunk_subset.shape() == chunk_representation.shape_u64() {
            self.async_retrieve_chunk_into_array_view_opt(chunk_indices, array_view, options)
                .await
        } else {
            let storage_handle = Arc::new(StorageHandle::new(self.storage.clone()));
            let storage_transformer = self
                .storage_transformers()
                .create_async_readable_transformer(storage_handle);
            let input_handle = Box::new(AsyncStoragePartialDecoder::new(
                storage_transformer,
                data_key(self.path(), chunk_indices, self.chunk_key_encoding()),
            ));

            self.codecs()
                .async_partial_decoder(input_handle, &chunk_representation, options)
                .await?
                .partial_decode_into_array_view_opt(chunk_subset, array_view, options)
                .await
                .map_err(ArrayError::CodecError)
        }
    }

    /// Async variant of [`partial_decoder_opt`](Array::partial_decoder_opt).
    #[allow(clippy::missing_errors_doc)]
    pub async fn async_partial_decoder_opt<'a>(
        &'a self,
        chunk_indices: &[u64],
        options: &CodecOptions,
    ) -> Result<Box<dyn AsyncArrayPartialDecoderTraits + 'a>, ArrayError> {
        let storage_handle = Arc::new(StorageHandle::new(self.storage.clone()));
        let storage_transformer = self
            .storage_transformers()
            .create_async_readable_transformer(storage_handle);
        let input_handle = Box::new(AsyncStoragePartialDecoder::new(
            storage_transformer,
            data_key(self.path(), chunk_indices, self.chunk_key_encoding()),
        ));
        let chunk_representation = self.chunk_array_representation(chunk_indices)?;
        Ok(self
            .codecs()
            .async_partial_decoder(input_handle, &chunk_representation, options)
            .await?)
    }
}
