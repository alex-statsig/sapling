/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#include "eden/fs/config/CheckoutConfig.h"

#include <cpptoml.h>

#include <folly/Range.h>
#include <folly/String.h>
#include <folly/io/Cursor.h>
#include <folly/io/IOBuf.h>
#include <folly/json.h>
#include "eden/fs/utils/FileUtils.h"
#include "eden/fs/utils/PathMap.h"

using folly::ByteRange;
using folly::IOBuf;
using folly::StringPiece;

namespace {
// TOML config file for the individual client.
const facebook::eden::RelativePathPiece kCheckoutConfig{"config.toml"};

// Keys for the TOML config file.
constexpr folly::StringPiece kRepoSection{"repository"};
constexpr folly::StringPiece kRepoSourceKey{"path"};
constexpr folly::StringPiece kRepoTypeKey{"type"};
constexpr folly::StringPiece kRepoCaseSensitiveKey{"case-sensitive"};
constexpr folly::StringPiece kMountProtocol{"protocol"};
constexpr folly::StringPiece kRequireUtf8Path{"require-utf8-path"};
constexpr folly::StringPiece kEnableTreeOverlay{"enable-tree-overlay"};
#ifdef _WIN32
constexpr folly::StringPiece kRepoGuid{"guid"};
#endif

#ifdef _WIN32
constexpr folly::StringPiece kMountProtocolPrjfs{"prjfs"};
#else
constexpr folly::StringPiece kMountProtocolFuse{"fuse"};
#endif
constexpr folly::StringPiece kMountProtocolNFS{"nfs"};

#ifdef _WIN32
constexpr folly::StringPiece kMountProtocolDefault{kMountProtocolPrjfs};
#else
constexpr folly::StringPiece kMountProtocolDefault{kMountProtocolFuse};
#endif

// Files of interest in the client directory.
const facebook::eden::RelativePathPiece kSnapshotFile{"SNAPSHOT"};
const facebook::eden::RelativePathPiece kOverlayDir{"local"};

// File holding mapping of client directories.
const facebook::eden::RelativePathPiece kClientDirectoryMap{"config.json"};

// Constants for use with the SNAPSHOT file
//
// - 4 byte identifier: "eden"
// - 4 byte format version number (big endian)
//
// Followed by:
// Version 1:
// - 20 byte commit ID
// - (Optional 20 byte commit ID, only present when there are 2 parents)
// Version 2:
// - 32-bit length
// - Arbitrary-length binary string of said length
constexpr folly::StringPiece kSnapshotFileMagic{"eden"};
enum : uint32_t {
  kSnapshotHeaderSize = 8,
  kSnapshotFormatVersion1 = 1,
  kSnapshotFormatVersion2 = 2,
};
} // namespace

namespace facebook {
namespace eden {

CheckoutConfig::CheckoutConfig(
    AbsolutePathPiece mountPath,
    AbsolutePathPiece clientDirectory)
    : clientDirectory_(clientDirectory), mountPath_(mountPath) {}

Hash CheckoutConfig::getParentCommit() const {
  // Read the snapshot.
  auto snapshotFile = getSnapshotPath();
  auto snapshotFileContents = readFile(snapshotFile).value();

  StringPiece contents{snapshotFileContents};

  if (contents.size() < kSnapshotHeaderSize) {
    throw std::runtime_error(folly::sformat(
        "eden SNAPSHOT file is too short ({} bytes): {}",
        contents.size(),
        snapshotFile));
  }

  if (!contents.startsWith(kSnapshotFileMagic)) {
    throw std::runtime_error(
        folly::sformat("unsupported legacy SNAPSHOT file"));
  }

  IOBuf buf(IOBuf::WRAP_BUFFER, ByteRange{contents});
  folly::io::Cursor cursor(&buf);
  cursor += kSnapshotFileMagic.size();
  auto version = cursor.readBE<uint32_t>();
  auto sizeLeft = cursor.length();
  switch (version) {
    case kSnapshotFormatVersion1: {
      if (sizeLeft != Hash::RAW_SIZE && sizeLeft != (Hash::RAW_SIZE * 2)) {
        throw std::runtime_error(folly::sformat(
            "unexpected length for eden SNAPSHOT file ({} bytes): {}",
            contents.size(),
            snapshotFile));
      }

      Hash parent1;
      cursor.pull(parent1.mutableBytes().data(), Hash::RAW_SIZE);

      if (!cursor.isAtEnd()) {
        // This is never used by EdenFS.
        Hash secondParent;
        cursor.pull(secondParent.mutableBytes().data(), Hash::RAW_SIZE);
      }

      return parent1;
    }

    case kSnapshotFormatVersion2: {
      auto bodyLength = cursor.readBE<uint32_t>();

      // The remainder of the file is the root ID.
      std::string rootId = cursor.readFixedString(bodyLength);

      // For now, we only support Hash root IDs, but soon this will become
      // variable-width.
      if (rootId.size() == Hash::RAW_SIZE) {
        // The plan is for 20-byte root IDs to always be written as 40-byte
        // ASCII hex, but just in case, for backward and forward compatibility,
        // handle the case that it was written as 20 byte binary.
        return Hash{folly::ByteRange{folly::StringPiece{rootId}}};
      } else if (rootId.size() == Hash::RAW_SIZE * 2) {
        return Hash{folly::StringPiece{rootId}};
      } else {
        throw std::runtime_error(folly::sformat(
            "SNAPSHOT file parent ID must be 20 or 40 bytes: was {} bytes",
            rootId.size()));
      }
    }

    default:
      throw std::runtime_error(folly::sformat(
          "unsupported eden SNAPSHOT file format (version {}): {}",
          uint32_t{version},
          snapshotFile));
  }
}

void CheckoutConfig::setParentCommit(Hash parent) const {
  std::array<uint8_t, kSnapshotHeaderSize + (2 * Hash::RAW_SIZE)> buffer;
  IOBuf buf(IOBuf::WRAP_BUFFER, ByteRange{buffer});
  folly::io::RWPrivateCursor cursor{&buf};

  // Snapshot file format:
  // 4-byte identifier: "eden"
  cursor.push(ByteRange{kSnapshotFileMagic});
  // 4-byte format version identifier
  cursor.writeBE<uint32_t>(kSnapshotFormatVersion1);
  // 20-byte commit ID: parent1
  cursor.push(parent.getBytes());
  // Older versions of EdenFS would write a second 20-byte hash here to track
  // the second HG parent commit, but it was never used for anything. Optional
  // 20-byte commit ID: parent2
  size_t writtenSize = cursor - folly::io::RWPrivateCursor{&buf};
  ByteRange snapshotData{buffer.data(), writtenSize};
  writeFileAtomic(getSnapshotPath(), snapshotData).value();
}

const AbsolutePath& CheckoutConfig::getClientDirectory() const {
  return clientDirectory_;
}

CaseSensitivity CheckoutConfig::getCaseSensitive() const {
  return caseSensitive_;
}

AbsolutePath CheckoutConfig::getSnapshotPath() const {
  return clientDirectory_ + kSnapshotFile;
}

AbsolutePath CheckoutConfig::getOverlayPath() const {
  return clientDirectory_ + kOverlayDir;
}

std::unique_ptr<CheckoutConfig> CheckoutConfig::loadFromClientDirectory(
    AbsolutePathPiece mountPath,
    AbsolutePathPiece clientDirectory) {
  // Extract repository name from the client config file
  auto configPath = clientDirectory + kCheckoutConfig;
  auto configRoot = cpptoml::parse_file(configPath.c_str());

  // Construct CheckoutConfig object
  auto config = std::make_unique<CheckoutConfig>(mountPath, clientDirectory);

  // Load repository information
  auto repository = configRoot->get_table(kRepoSection.str());
  config->repoType_ = *repository->get_as<std::string>(kRepoTypeKey.str());
  config->repoSource_ = *repository->get_as<std::string>(kRepoSourceKey.str());

  auto mountProtocol = repository->get_as<std::string>(kMountProtocol.str())
                           .value_or(kMountProtocolDefault);
  config->mountProtocol_ = mountProtocol == kMountProtocolNFS
      ? MountProtocol::NFS
      : (folly::kIsWindows ? MountProtocol::PRJFS : MountProtocol::FUSE);

  // Read optional case-sensitivity.
  auto caseSensitive = repository->get_as<bool>(kRepoCaseSensitiveKey.str());
  config->caseSensitive_ = caseSensitive
      ? static_cast<CaseSensitivity>(*caseSensitive)
      : kPathMapDefaultCaseSensitive;

  auto requireUtf8Path = repository->get_as<bool>(kRequireUtf8Path.str());
  config->requireUtf8Path_ = requireUtf8Path ? *requireUtf8Path : true;

  auto enableTreeOverlay = repository->get_as<bool>(kEnableTreeOverlay.str());
  config->enableTreeOverlay_ = enableTreeOverlay.value_or(false);

#ifdef _WIN32
  auto guid = repository->get_as<std::string>(kRepoGuid.str());
  config->repoGuid_ = guid ? Guid{*guid} : Guid::generate();
#endif

  return config;
}

folly::dynamic CheckoutConfig::loadClientDirectoryMap(
    AbsolutePathPiece edenDir) {
  // Extract the JSON and strip any comments.
  auto configJsonFile = edenDir + kClientDirectoryMap;
  auto jsonContents = readFile(configJsonFile).value();
  auto jsonWithoutComments = folly::json::stripComments(jsonContents);
  if (jsonWithoutComments.empty()) {
    return folly::dynamic::object();
  }

  // Parse the comment-free JSON while tolerating trailing commas.
  folly::json::serialization_opts options;
  options.allow_trailing_comma = true;
  return folly::parseJson(jsonWithoutComments, options);
}
} // namespace eden
} // namespace facebook
