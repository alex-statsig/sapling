/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

//! Errors.

use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ErrorKind {
    #[error("the provided store file is not a valid store file: {0}")]
    NotAStoreFile(PathBuf),
    #[error("tree version not supported: {0}")]
    UnsupportedTreeVersion(u32),
    #[error("store file version not supported: {0}")]
    UnsupportedVersion(u32),
    #[error("invalid store id: {0}")]
    InvalidStoreId(u64),
    #[error("store is read-only")]
    ReadOnlyStore,
    #[error("treedirstate is corrupt")]
    CorruptTree,
    #[error("callback error: {0}")]
    CallbackError(String),
    #[error("dirstate/treestate was out of date and therefore did not flush")]
    TreestateOutOfDate,
    #[error("timed out waiting for working copy lock")]
    LockTimeout,
}
