# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License found in the LICENSE file in the root
# directory of this source tree.

  $ CACHEDIR=$PWD/cachepath
  $ . "${TEST_FIXTURES}/library.sh"

Setup config
  $ REPOTYPE="blob_files"
  $ export LFS_THRESHOLD="1000"
  $ setup_common_config $REPOTYPE
  $ cd $TESTTMP

Setup repo
  $ hginit_treemanifest repo-hg-nolfs
  $ cd repo-hg-nolfs
  $ setup_hg_server

# Commit small file and blobimport
  $ echo s > small
  $ hg commit -Aqm "add small"

  $ hg bookmark master_bookmark -r tip

  $ cd ..
  $ blobimport repo-hg-nolfs/.hg repo

Setup Mononoke
  $ start_and_wait_for_mononoke_server
Setup LFS server
  $ lfs_uri="$(lfs_server)/repo"

Clone the repository, then enable LFS
  $ hgclone_treemanifest ssh://user@dummy/repo-hg-nolfs repo-hg-lfs --noupdate --config extensions.remotenames=
  $ cd repo-hg-lfs
  $ setup_hg_client
  $ setup_hg_modern_lfs "$lfs_uri" 1000B "$TESTTMP/lfs-cache1"

  $ cat >> .hg/hgrc <<EOF
  > [extensions]
  > pushrebase =
  > remotenames =
  > EOF

  $ hgmn pull -q
  $ hgmn update -r master_bookmark -q

Submit a large file
  $ yes 2>/dev/null | head -c 2000 > large
  $ hg commit -Aqm "add large"
  $ hg cp large largeCopy
  $ hg mv large largeNew
  $ hg commit -Aqm "copy and move large"
  $ hgmn push -q -r . --to master_bookmark

Create a new repository, enable LFS there as well
  $ hgclone_treemanifest ssh://user@dummy/repo-hg-nolfs repo-hg-lfs2 --noupdate --config extensions.remotenames=
  $ cd repo-hg-lfs2
  $ setup_hg_client
  $ setup_hg_modern_lfs "$lfs_uri" 1000B "$TESTTMP/lfs-cache2"

  $ cat >> .hg/hgrc <<EOF
  > [extensions]
  > pushrebase =
  > remotenames =
  > [remotefilelog]
  > getpackversion = 2
  > EOF

Pull changes from Mononoke
  $ hgmn pull -q

  $ hgmn update -r master_bookmark
  3 files updated, 0 files merged, 0 files removed, 0 files unresolved

  $ hgmn st --change . -C
  A largeCopy
    large
  A largeNew
    large
  R large

  $ hg debugfilerevision
  ee97b40ee584: copy and move large
   largeCopy: bin=1 lnk=0 flag=0 size=2000 copied='large'
   largeNew: bin=1 lnk=0 flag=0 size=2000 copied='large'
