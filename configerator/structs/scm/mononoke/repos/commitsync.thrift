// @generated SignedSource<<9837dad9c30b88e2ae3bdb2545065bbe>>
// DO NOT EDIT THIS FILE MANUALLY!
// This file is a mechanical copy of the version in the configerator repo. To
// modify it, edit the copy in the configerator repo instead and copy it over by
// running the following in your fbcode directory:
//
// configerator-thrift-updater scm/mononoke/repos/commitsync.thrift
/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

include "configerator/structs/scm/mononoke/repos/repos.thrift"

namespace py configerator.mononoke.commitsync

typedef string LargeRepoName

/// Config that doesn't change from version to version
struct RawSmallRepoPermanentConfig {
  /// Bookmark prefix to use for each small repo bookmark
  /// when it's remapped to a large repo (except for common_pushrebase_bookmarks)
  1: string bookmark_prefix;
  /// Mapping from each common_pushrebase_bookmark in the large repo to
  /// the equivalent bookmark in the small repo.
  /// This allows using a different bookmark name for the common pushrebase bookmark
  /// between the large repos and some of the small repos (e.g: a small repo imported
  /// from git may want to sync its `heads/master` to `master` in a large repo)
  2: optional map<string, string> common_pushrebase_bookmarks_map;
} (rust.exhaustive)

/// Config that applies for all versions for a given large repo
struct CommonCommitSyncConfig {
  /// Mapping from  small repos id to their parameters
  1: map<i32, RawSmallRepoPermanentConfig> small_repos;
  /// Bookmarks that have the same name in small and large repos
  /// and that are possible to pushrebase to from small repos.
  2: list<string> common_pushrebase_bookmarks;
  /// Id of the large repo
  3: i32 large_repo_id;
} (rust.exhaustive)

struct RawCommitSyncConfigAllVersionsOneRepo {
  /// All versions of `RawCommitSyncConfig` ever present for a given repo
  1: list<repos.RawCommitSyncConfig> versions;
  /// Current version of `RawCommitSyncConfig` used by a given repo
  /// DEPRECATED, WILL BE DELETED SOON
  2: string current_version;
  /// Common config that applies to all versions
  3: CommonCommitSyncConfig common;
} (rust.exhaustive)

struct RawCommitSyncAllVersions {
  /// All versions of `RawCommitSyncConfig` for all known repos
  1: map<LargeRepoName, RawCommitSyncConfigAllVersionsOneRepo> repos;
} (rust.exhaustive)

/// Current versions of commit sync maps for all known repos
struct RawCommitSyncCurrentVersions {
  1: map_LargeRepoName_RawCommitSyncConfig_4235 repos;
} (rust.exhaustive)

// The following were automatically generated and may benefit from renaming.
typedef map<LargeRepoName, repos.RawCommitSyncConfig> (
  rust.type = "HashMap",
) map_LargeRepoName_RawCommitSyncConfig_4235
