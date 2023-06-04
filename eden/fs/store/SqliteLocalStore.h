/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#pragma once
#include <folly/Synchronized.h>
#include "eden/fs/sqlite/SqliteDatabase.h"
#include "eden/fs/store/LocalStore.h"

namespace facebook::eden {

class EdenStats;

using EdenStatsPtr = RefPtr<EdenStats>;

/** An implementation of LocalStore that stores values in Sqlite.
 * SqliteLocalStore is thread safe, allowing reads and writes from
 * any thread.
 * */
class SqliteLocalStore final : public LocalStore {
 public:
  explicit SqliteLocalStore(AbsolutePathPiece pathToDb, EdenStatsPtr edenStats);
  void open() override;
  void close() override;
  void clearKeySpace(KeySpace keySpace) override;
  void compactKeySpace(KeySpace keySpace) override;
  StoreResult get(KeySpace keySpace, folly::ByteRange key) const override;
  bool hasKey(KeySpace keySpace, folly::ByteRange key) const override;
  void put(KeySpace keySpace, folly::ByteRange key, folly::ByteRange value)
      override;
  std::unique_ptr<LocalStore::WriteBatch> beginWrite(
      size_t bufSize = 0) override;

 private:
  mutable SqliteDatabase db_;
};

} // namespace facebook::eden
