/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#include "eden/fs/model/git/GitBlob.h"

#include <folly/Conv.h>
#include <folly/io/Cursor.h>
#include <folly/io/IOBuf.h>
#include "eden/fs/model/Blob.h"

using folly::IOBuf;
using std::invalid_argument;

namespace facebook::eden {

BlobPtr deserializeGitBlob(const IOBuf* data) {
  folly::io::Cursor cursor(data);

  // Find the end of the header and extract the size.
  if (cursor.readFixedString(5) != "blob ") {
    throw invalid_argument("Contents did not start with expected header.");
  }

  // 25 characters is long enough to represent any legitimate length
  size_t maxSizeLength = 25;
  auto sizeStr = cursor.readTerminatedString('\0', maxSizeLength);
  auto contentSize = folly::to<unsigned int>(sizeStr);
  if (contentSize != cursor.length()) {
    throw invalid_argument("Size in header should match contents");
  }

  // If we have a managed IOBuf, we can clone it without copying the
  // underlying data.  Otherwise we need to make a full copy of the data.
  //
  // TODO: We probably should add a Cursor::managedClone() function that does
  // this for us.
  IOBuf contents;
  if (data->isManaged()) {
    cursor.clone(contents, contentSize);
  } else {
    contents = IOBuf(IOBuf::CREATE, contentSize);
    while (true) {
      auto nextChunk = cursor.peekBytes();
      if (nextChunk.empty()) {
        break;
      }
      memcpy(contents.writableData(), nextChunk.data(), nextChunk.size());
      contents.append(nextChunk.size());
      cursor.skip(nextChunk.size());
    }
  }

  return std::make_shared<BlobPtr::element_type>(std::move(contents));
}

} // namespace facebook::eden
