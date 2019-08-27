// Copyright 2018 Facebook, Inc.
//
// This software may be used and distributed according to the terms of the
// GNU General Public License version 2 or any later version.

//! Integrity check support for `index`.
//!
//! See [`ChecksumTable`] for details.

// Format details:
//
// ```plain,ignore
// SUM_FILE := CHUNK_SIZE_LOG (u64, BE) + END_OFFSET (u64, BE) + CHECKSUM_LIST
// CHECKSUM_LIST := "" | CHECKSUM_LIST + CHUNK_CHECKSUM (u64, BE)
// ```
//
// The "atomic-replace" part could be a scaling issue if the checksum
// table grows too large, or has frequent small updates. For those cases,
// it's better to build the checksum-related logic inside the source of
// truth file format directly.
//
// Inside `indexedlog` crate, `ChecksumTable` is mainly used for indexes,
// which are relatively small comparing to their source of truth, and
// infrequently updated, and are already complex that it's cleaner to not
// embed checksum logic into them.

use crate::errors::{self, data_error, parameter_error};
use crate::utils::{atomic_write, mmap_readonly, xxhash};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use failure::Fallible;
use fs2::FileExt;
use memmap::Mmap;
use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};

/// An table of checksums to verify another file.
///
/// The table is backed by a file on disk, and can be updated incrementally
/// for append-only files.
///
/// To use [`ChecksumTable`], make sure:
/// - Before reading, call [ChecksumTable::check_range] to verify a range.
/// - After appending to the source of truth, call [`ChecksumTable::update`].
///
/// [`ChecksumTable`] is only designed to support append-only files efficiently.
/// However, it could also be used for non-append-only files in a less efficient
/// way by always using [`ChecksumTable::clear`] to reset the existing table
/// before updating.
pub struct ChecksumTable {
    // The file to be checked. Maintain a separate mmap buffer so
    // the API is easier to use for the caller. It's expected for
    // the caller to also use mmap to let the system do the "sharing"
    // work. But that's not required for correctness.
    file: File,
    buf: Mmap,
    path: PathBuf,

    // Whether fsync is set.
    fsync: bool,

    // The checksum file
    checksum_path: PathBuf,
    chunk_size_log: u32,
    end: u64,
    checksums: Vec<u64>,

    // A bitvec about What chunks are checked.
    // Using internal mutability so exposed APIs do not need "mut".
    checked: RefCell<Vec<u64>>,
}

/// Append extra extension to a Path
fn path_appendext(path: &Path, ext: &str) -> PathBuf {
    let mut buf = path.to_path_buf();
    match path.extension() {
        Some(x) => {
            let mut s = x.to_os_string();
            s.push(".");
            s.push(ext);
            buf.set_extension(s);
        }
        None => {
            buf.set_extension(ext);
        }
    };
    buf
}

/// Default chunk size: 1MB
const DEFAULT_CHUNK_SIZE_LOG: u32 = 20;
/// Max chunk size: 2GB
const MAX_CHUNK_SIZE_LOG: u32 = 31;

impl ChecksumTable {
    /// Check given byte range. Return `Ok(())` if the byte range passes checksum,
    /// raise `ChecksumError` if it fails or unsure.
    ///
    /// Depending on `chunk_size_log`, `ChecksumError` might be caused by
    /// something within a same chunk, but outside the provided range being
    /// broken, or if the range is outside what the checksum table covers.
    pub fn check_range(&self, offset: u64, length: u64) -> Fallible<()> {
        // Empty range is treated as good.
        if length == 0 {
            return Ok(());
        }

        // Ranges not covered by checksums are treated as bad.
        if offset + length > self.end {
            return checksum_error(self, offset, length);
        }

        // Otherwise, scan related chunks.
        let start = (offset >> self.chunk_size_log) as usize;
        let end = ((offset + length - 1) >> self.chunk_size_log) as usize;
        if !(start..(end + 1)).all(|i| self.check_chunk(i)) {
            return checksum_error(self, offset, length);
        }
        Ok(())
    }

    fn check_chunk(&self, index: usize) -> bool {
        let mut checked = self.checked.borrow_mut();
        if (checked[index / 64] >> (index % 64)) & 1 == 1 {
            true
        } else {
            let start = index << self.chunk_size_log;
            let end = (self.end as usize).min((index + 1) << self.chunk_size_log);
            if start == end {
                return true;
            }
            let hash = xxhash(&self.buf[start..end]);
            if hash == self.checksums[index] {
                checked[index / 64] |= 1 << (index % 64);
                true
            } else {
                false
            }
        }
    }

    /// Construct [`ChecksumTable`] for checking the given path.
    ///
    /// The checksum table uses a separate file name: `path + ".sum"`. If
    /// that file exists, load the table from it. Otherwise, the table
    /// is empty and [ChecksumTable::check_range] will return `false`
    /// for all non-empty range.
    ///
    /// Once the table is loaded from disk, changes on disk won't affect
    /// the table in memory.
    pub fn new<P: AsRef<Path>>(path: &P) -> Fallible<Self> {
        // Read the source of truth file as a mmap buffer
        let file = OpenOptions::new().read(true).open(path)?;
        let (mmap, len) = mmap_readonly(&file, None)?;

        // Read checksum file into memory
        let checksum_path = path_appendext(path.as_ref(), "sum");
        let mut checksum_buf = Vec::new();
        match OpenOptions::new().read(true).open(&checksum_path) {
            Ok(mut checksum_file) => {
                checksum_file.read_to_end(&mut checksum_buf)?;
            }
            Err(err) => {
                if err.kind() != io::ErrorKind::NotFound {
                    return Err(err.into());
                }
            }
        }

        // Parse checksum file
        let (chunk_size_log, chunk_end, checksums, checked) = if checksum_buf.len() == 0 {
            (DEFAULT_CHUNK_SIZE_LOG, 0, vec![], vec![])
        } else {
            let mut cur = Cursor::new(checksum_buf);
            let chunk_size_log = cur.read_u64::<LittleEndian>()?;
            if chunk_size_log > MAX_CHUNK_SIZE_LOG as u64 {
                let msg = format!(
                    "invalid chunk_size_log {:?} when opening {:?} for checksum",
                    chunk_size_log,
                    &path.as_ref()
                );
                return Err(data_error(msg));
            }
            let chunk_size_log = chunk_size_log as u32;
            let chunk_size = 1 << chunk_size_log;
            let file_size = len.min(cur.read_u64::<LittleEndian>()?);
            let n = (file_size + chunk_size - 1) / chunk_size;
            let mut checksums = Vec::with_capacity(n as usize);
            for _ in 0..n {
                checksums.push(cur.read_u64::<LittleEndian>()?);
            }
            let checked = vec![0; (n as usize + 63) / 64];
            (chunk_size_log, file_size, checksums, checked)
        };

        Ok(ChecksumTable {
            file,
            buf: mmap,
            path: path.as_ref().to_path_buf(),
            fsync: false,
            chunk_size_log,
            end: chunk_end,
            checksum_path,
            checksums,
            checked: RefCell::new(checked),
        })
    }

    /// Set fsync behavior.
    ///
    /// If true, then [`ChecksumTable::update`] will use `fsync` to make
    /// sure data reaches the physical device before returning.
    pub fn fsync(mut self, fsync: bool) -> Self {
        self.fsync = fsync;
        self
    }

    /// Clone the checksum table.
    pub fn clone(&self) -> Fallible<Self> {
        let file = self.file.duplicate()?;
        let mmap = mmap_readonly(&file, (self.buf.len() as u64).into())?.0;
        Ok(ChecksumTable {
            file,
            buf: mmap,
            path: self.path.clone(),
            fsync: self.fsync,
            checksum_path: self.checksum_path.clone(),
            chunk_size_log: self.chunk_size_log,
            end: self.end,
            checksums: self.checksums.clone(),
            checked: self.checked.clone(),
        })
    }

    /// Update the checksum table and write it back to disk.
    ///
    /// `chunk_size_log` decides the chunk size: `1 << chunk_size_log`.
    ///
    /// If `chunk_size_log` is `None`, will reuse the existing `chunk_size_log`
    /// specified by the checksum table, or a default value if the table is
    /// empty (ex. newly created via `new`).
    ///
    /// If `chunk_size_log` differs from the existing one, the table will be
    /// rebuilt from scratch.  Otherwise it's updated incrementally.
    ///
    /// For any part in the old table that will be rewritten, checksum
    /// verification will be preformed on them. Returns `InvalidData` error if
    /// that verification fails.
    ///
    /// Otherwise, update the in-memory checksum table. Then write it in an
    /// atomic-replace way.  Return write errors if write fails.
    pub fn update(&mut self, chunk_size_log: Option<u32>) -> Fallible<()> {
        let (mmap, len) = mmap_readonly(&self.file, None)?;
        let chunk_size_log = chunk_size_log.unwrap_or(self.chunk_size_log);
        if chunk_size_log > MAX_CHUNK_SIZE_LOG {
            return Err(parameter_error("chunk_size_log is too largee"));
        }
        let chunk_size = 1 << chunk_size_log;
        let old_chunk_size = 1 << self.chunk_size_log;

        if chunk_size == 0 {
            return Err(parameter_error("chunk_size_log cannot be 0"));
        }

        if len == self.end && chunk_size == old_chunk_size {
            return Ok(());
        }

        if len < self.end {
            // Breaks the "append-only" assumption.
            return Err(errors::path_data_error(
                &self.path,
                "file was truncated unexpectedly",
            ));
        }

        let mut checksums = self.checksums.clone();
        if chunk_size == old_chunk_size {
            if self.end % chunk_size != 0 {
                // The last block need recalculate
                checksums.pop();
            }
        } else {
            // Recalculate everything
            checksums.clear();
        };

        // Before recalculating, verify the changed chunks first.
        let start = checksums.len() as u64 * old_chunk_size;
        self.check_range(start, self.end - start)?;

        let mut offset = checksums.len() as u64 * chunk_size;
        while offset < len {
            let end = (offset + chunk_size).min(len);
            let chunk = &mmap[offset as usize..end as usize];
            checksums.push(xxhash(chunk));
            offset = end;
        }

        // Prepare changes
        let mut buf = vec![];
        buf.write_u64::<LittleEndian>(chunk_size_log as u64)?;
        buf.write_u64::<LittleEndian>(len)?;
        for checksum in &checksums {
            buf.write_u64::<LittleEndian>(*checksum)?;
        }

        // Write changes to disk
        atomic_write(&self.checksum_path, &buf, self.fsync)?;

        // Update fields
        self.buf = mmap;
        self.end = len;
        self.checked = RefCell::new(vec![0u64; (checksums.len() + 63) / 64]);
        self.chunk_size_log = 63 - (chunk_size as u64).leading_zeros();
        self.checksums = checksums;

        Ok(())
    }

    /// Reset the table as if it's recreated from an empty file. Do not write to
    /// disk immediately.
    ///
    /// Usually this is followed by [`ChecksumTable::update`].
    pub fn clear(&mut self) {
        self.end = 0;
        self.checksums = vec![];
        self.checked = RefCell::new(vec![]);
    }
}

// Intentionally not inlined. This affects the "index lookup (disk, verified)"
// benchmark. It takes 74ms with this function inlined, and 61ms without.
//
// Reduce instruction count in `Index::verify_checksum`.
#[inline(never)]
fn checksum_error(table: &ChecksumTable, offset: u64, length: u64) -> Fallible<()> {
    Err(errors::ChecksumError {
        path: table.path.to_string_lossy().to_string(),
        start: offset,
        end: offset + length,
    }
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Seek, SeekFrom, Write};
    use tempfile::tempdir;

    fn setup() -> (File, Box<dyn Fn() -> Fallible<ChecksumTable>>) {
        let dir = tempdir().unwrap();

        // Checksum an non-existed file is an error.
        assert!(ChecksumTable::new(&dir.path().join("non-existed")).is_err());

        // Checksum an empty file is not an error.
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&dir.path().join("main"))
            .expect("open");
        let f = move || ChecksumTable::new(&dir.path().join("main"));

        (file, Box::new(f))
    }

    #[test]
    fn test_non_existed() {
        // Checksum an non-existed file is an error.
        let dir = tempdir().unwrap();
        assert!(ChecksumTable::new(&dir.path().join("non-existed")).is_err());
    }

    #[test]
    fn test_empty() {
        let (_file, get_table) = setup();
        let table = get_table().expect("checksum on an empty file is okay");
        assert!(table.check_range(0, 0).is_ok());
        assert!(table.check_range(0, 1).is_err());
        assert!(table.check_range(1, 0).is_ok());
        assert!(table.check_range(1, 1).is_err());
    }

    #[test]
    fn test_update_from_empty() {
        let (mut file, get_table) = setup();
        file.write_all(b"01234567890123456789").expect("write");
        let mut table = get_table().unwrap();
        table.update(7.into()).expect("update");
        assert!(table.check_range(1, 19).is_ok());
        assert!(table.check_range(1, 20).is_err());
        assert!(table.check_range(19, 1).is_ok());
        assert!(table.check_range(0, 1).is_ok());
        assert!(table.check_range(0, 21).is_err());
    }

    #[test]
    fn test_incremental_update() {
        let (mut file, get_table) = setup();
        file.write_all(b"01234567890123456789").expect("write");
        let mut table = get_table().unwrap();
        table.update(3.into()).expect("update");
        assert!(table.check_range(0, 20).is_ok());
        file.write_all(b"01234567890123456789").expect("write");
        assert!(table.check_range(20, 1).is_err());
        table.update(None).expect("update");
        assert!(table.check_range(20, 20).is_ok());
    }

    #[test]
    fn test_change_chunk_size() {
        let (mut file, get_table) = setup();
        file.write_all(b"01234567890123456789").expect("write");
        let mut table = get_table().unwrap();
        table.update(2.into()).expect("update");
        for &chunk_size in &[1, 2, 3, 4] {
            table.update(chunk_size.into()).expect("update");
            assert!(table.check_range(0, 20).is_ok());
            assert!(table.check_range(0, 21).is_err());
        }
    }

    #[test]
    fn test_reload_from_disk() {
        let (mut file, get_table) = setup();
        file.write_all(b"01234567890123456789").expect("write");
        let mut table = get_table().unwrap();
        table.update(3.into()).expect("update");
        assert!(table.check_range(0, 20).is_ok());
        assert!(table.check_range(0, 21).is_err());
        let table = get_table().unwrap();
        assert!(table.check_range(0, 20).is_ok());
        assert!(table.check_range(0, 21).is_err());
    }

    #[test]
    fn test_broken_byte() {
        let (mut file, get_table) = setup();
        file.write_all(b"01234567890123456789").expect("write");
        let mut table = get_table().unwrap();
        table.update(1.into()).expect("update");
        // Corruption: Corrupt the file at byte 5
        file.seek(SeekFrom::Start(5)).expect("seek");
        file.write_all(&[1]).expect("write");
        let err = table.check_range(0, 10).unwrap_err();
        assert!(format!("{}", err).ends_with("range 0..10 failed checksum check"));
        assert!(table.check_range(5, 1).is_err());
        // Byte 4 is not corrupted. But the same chunk is corrupted.
        assert!(table.check_range(4, 1).is_err());
        assert!(table.check_range(7, 13).is_ok());
        assert!(table.check_range(0, 4).is_ok());
    }

    // This test truncates mmaped files which is unsupported by Windows.
    #[cfg(not(windows))]
    #[test]
    fn test_truncate() {
        let (mut file, get_table) = setup();
        file.write_all(b"01234567890123456789").expect("write");
        let mut table = get_table().unwrap();
        table.update(1.into()).expect("update");
        file.set_len(19).expect("set_len");
        let table = get_table().unwrap();
        assert!(table.check_range(0, 20).is_err());
        assert!(table.check_range(0, 19).is_err());
        assert!(table.check_range(0, 18).is_ok());
    }

    #[test]
    fn test_broken_during_update() {
        let (mut file, get_table) = setup();
        file.write_all(b"01234567890123456789").expect("write");
        let mut table = get_table().unwrap();
        table.update(3.into()).expect("update");
        file.seek(SeekFrom::End(-1)).expect("seek");
        file.write_all(b"x0123").expect("write");
        table.update(None).expect_err("broken during update");
        table.update(3.into()).expect_err("broken during update");
        // With clear(), update can work.
        table.clear();
        table.update(3.into()).expect("update");
        // If chunk boundary aligns with the broken range, corruption won't be detected.
        assert_eq!(file.seek(SeekFrom::End(-1)).expect("seek"), 23);
        file.write_all(b"x123451234512345").expect("write");
        table.update(None).expect("update");
        // But explicitly verifying it will reveal the problem.
        assert!(table.check_range(23, 1).is_err());
        // Update with a different chunk_size will also cause an error.
        table.update(2.into()).expect_err("broken during update");
    }
}
