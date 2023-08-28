/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#pragma once
#ifdef _WIN32
#include <folly/portability/SysTypes.h> //@manual
#include <sys/stat.h>
#else
#include <dirent.h>
#endif
#include <sys/types.h>
#include <cstdint>

namespace facebook::eden {

/** Represents the type of a filesystem entry.
 *
 * This is the same type and intent as the d_type field of a dirent struct.
 *
 * We provide an explicit type to make it clearer when we're working
 * with this value.
 *
 * https://www.daemon-systems.org/man/DTTOIF.3.html
 *
 * Portability note: Solaris does not have a d_type field, so this
 * won't compile.  We don't currently have plans to support Solaris.
 */

#ifdef _WIN32
/**
 * Convertion between st_mode and d_type on Windows. On Windows the 4th nibble
 * of mode contains the type of directory entry. Right shifting by 12 bits to
 * form a d_type.
 */
static_assert(S_IFMT == 0xF000, "The S_IFMT on Windows should be 0xF000");
#define POSIX_BIT_SHIFT 12

#define DT_UNKNOWN 0
#define DT_FIFO ((_S_IFIFO) >> POSIX_BIT_SHIFT)
#define DT_CHR ((_S_IFCHR) >> POSIX_BIT_SHIFT)
#define DT_DIR ((_S_IFDIR) >> POSIX_BIT_SHIFT)
#define DT_REG ((_S_IFREG) >> POSIX_BIT_SHIFT)

// Windows CRT does not define S_IFLNK and _S_IFSOCK. So we arbitrarily define
// it here.
#define S_IFLNK 0xA000
#define DT_LNK (S_IFLNK >> POSIX_BIT_SHIFT)

#define _S_IFSOCK 0xC000
#define DT_SOCK (_S_IFSOCK >> POSIX_BIT_SHIFT)

#define _S_IFBLK 0x3000
#define DT_BLK (_S_IFBLK >> POSIX_BIT_SHIFT)

#define IFTODT(mode) (((mode)&_S_IFMT) >> POSIX_BIT_SHIFT)
#define DTTOIF(type) (((type) << POSIX_BIT_SHIFT) & _S_IFMT)

#ifndef S_ISDIR
#define S_ISDIR(mode) ((mode >> POSIX_BIT_SHIFT) == DT_DIR)
#endif

#ifndef S_ISREG
#define S_ISREG(mode) ((mode >> POSIX_BIT_SHIFT) == DT_REG)
#endif

#ifndef S_ISBLK
#define S_ISBLK(mode) ((mode >> POSIX_BIT_SHIFT) == DT_BLK)
#endif

#ifndef S_ISCHR
#define S_ISCHR(mode) ((mode >> POSIX_BIT_SHIFT) == DT_CHR)
#endif

#ifndef S_ISFIFO
#define S_ISFIFO(mode) ((mode >> POSIX_BIT_SHIFT) == DT_FIFO)
#endif

#define S_ISSOCK(mode) ((mode >> POSIX_BIT_SHIFT) == DT_SOCK)
#define S_ISLNK(mode) ((mode >> POSIX_BIT_SHIFT) == DT_LNK)

/**
 * We only use d_type from dirent on Windows.
 */
struct dirent {
  unsigned char d_type;
};
#endif

enum class dtype_t : decltype(dirent::d_type) {
  Unknown = DT_UNKNOWN,
  Fifo = DT_FIFO,
  Char = DT_CHR,
  Dir = DT_DIR,
  Regular = DT_REG,
  Symlink = DT_LNK,
  Socket = DT_SOCK,
#ifndef _WIN32
  Block = DT_BLK,
  Whiteout = DT_WHT,
#endif
};

/// Convert to a form suitable for inserting into a stat::st_mode
inline mode_t dtype_to_mode(dtype_t dt) {
  return DTTOIF(static_cast<uint8_t>(dt));
}

/// Convert from stat::st_mode form to dirent::d_type form
inline dtype_t mode_to_dtype(mode_t mode) {
  return static_cast<dtype_t>(IFTODT(mode));
}
} // namespace facebook::eden
