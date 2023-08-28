/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

mod pendingchanges;

pub use pendingchanges::PendingChange;
pub use pendingchanges::PendingChanges;

#[derive(PartialEq)]
pub enum FileSystemType {
    Normal,
    Watchman,
    Eden,
}
