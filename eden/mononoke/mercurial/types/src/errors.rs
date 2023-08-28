/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MononokeHgError {
    #[error("invalid sha-1 input: {0}")]
    InvalidSha1Input(String),
    #[error("invalid fragment list: {0}")]
    InvalidFragmentList(String),
    #[error("invalid Thrift structure '{0}': {1}")]
    InvalidThrift(String, String),
    #[error("error while deserializing blob for '{0}'")]
    BlobDeserializeError(String),
    #[error("imposssible to parse unknown rev flags")]
    UnknownRevFlags,
}
