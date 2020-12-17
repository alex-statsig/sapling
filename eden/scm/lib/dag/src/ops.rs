/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

//! DAG and Id operations (mostly traits)

use crate::clone::CloneData;
use crate::default_impl;
use crate::id::Group;
use crate::id::Id;
use crate::id::VertexName;
use crate::locked::Locked;
use crate::namedag::MemNameDag;
use crate::nameset::id_lazy::IdLazySet;
use crate::nameset::id_static::IdStaticSet;
use crate::nameset::NameSet;
use crate::nameset::SyncNameSetQuery;
use crate::IdSet;
use crate::Result;
use std::sync::Arc;

/// DAG related read-only algorithms.
#[async_trait::async_trait]
pub trait DagAlgorithm: Send + Sync {
    /// Sort a `NameSet` topologically.
    async fn sort(&self, set: &NameSet) -> Result<NameSet>;

    /// Re-create the graph so it looks better when rendered.
    async fn beautify(&self, main_branch: Option<NameSet>) -> Result<MemNameDag> {
        default_impl::beautify(self, main_branch).await
    }

    /// Get ordered parent vertexes.
    async fn parent_names(&self, name: VertexName) -> Result<Vec<VertexName>>;

    /// Returns a [`SpanSet`] that covers all vertexes tracked by this DAG.
    async fn all(&self) -> Result<NameSet>;

    /// Calculates all ancestors reachable from any name from the given set.
    async fn ancestors(&self, set: NameSet) -> Result<NameSet>;

    /// Calculates parents of the given set.
    ///
    /// Note: Parent order is not preserved. Use [`NameDag::parent_names`]
    /// to preserve order.
    async fn parents(&self, set: NameSet) -> Result<NameSet> {
        default_impl::parents(self, set).await
    }

    /// Calculates the n-th first ancestor.
    async fn first_ancestor_nth(&self, name: VertexName, n: u64) -> Result<VertexName> {
        default_impl::first_ancestor_nth(self, name, n).await
    }

    /// Calculates heads of the given set.
    async fn heads(&self, set: NameSet) -> Result<NameSet> {
        default_impl::heads(self, set).await
    }

    /// Calculates children of the given set.
    async fn children(&self, set: NameSet) -> Result<NameSet>;

    /// Calculates roots of the given set.
    async fn roots(&self, set: NameSet) -> Result<NameSet> {
        default_impl::roots(self, set).await
    }

    /// Calculates one "greatest common ancestor" of the given set.
    ///
    /// If there are no common ancestors, return None.
    /// If there are multiple greatest common ancestors, pick one arbitrarily.
    /// Use `gca_all` to get all of them.
    async fn gca_one(&self, set: NameSet) -> Result<Option<VertexName>> {
        default_impl::gca_one(self, set).await
    }

    /// Calculates all "greatest common ancestor"s of the given set.
    /// `gca_one` is faster if an arbitrary answer is ok.
    async fn gca_all(&self, set: NameSet) -> Result<NameSet> {
        default_impl::gca_all(self, set).await
    }

    /// Calculates all common ancestors of the given set.
    async fn common_ancestors(&self, set: NameSet) -> Result<NameSet> {
        default_impl::common_ancestors(self, set).await
    }

    /// Tests if `ancestor` is an ancestor of `descendant`.
    async fn is_ancestor(&self, ancestor: VertexName, descendant: VertexName) -> Result<bool> {
        default_impl::is_ancestor(self, ancestor, descendant).await
    }

    /// Calculates "heads" of the ancestors of the given set. That is,
    /// Find Y, which is the smallest subset of set X, where `ancestors(Y)` is
    /// `ancestors(X)`.
    ///
    /// This is faster than calculating `heads(ancestors(set))` in certain
    /// implementations like segmented changelog.
    ///
    /// This is different from `heads`. In case set contains X and Y, and Y is
    /// an ancestor of X, but not the immediate ancestor, `heads` will include
    /// Y while this function won't.
    async fn heads_ancestors(&self, set: NameSet) -> Result<NameSet> {
        default_impl::heads_ancestors(self, set).await
    }

    /// Calculates the "dag range" - vertexes reachable from both sides.
    async fn range(&self, roots: NameSet, heads: NameSet) -> Result<NameSet>;

    /// Calculates `ancestors(reachable) - ancestors(unreachable)`.
    async fn only(&self, reachable: NameSet, unreachable: NameSet) -> Result<NameSet> {
        default_impl::only(self, reachable, unreachable).await
    }

    /// Calculates `ancestors(reachable) - ancestors(unreachable)`, and
    /// `ancestors(unreachable)`.
    /// This might be faster in some implementations than calculating `only` and
    /// `ancestors` separately.
    async fn only_both(
        &self,
        reachable: NameSet,
        unreachable: NameSet,
    ) -> Result<(NameSet, NameSet)> {
        default_impl::only_both(self, reachable, unreachable).await
    }

    /// Calculates the descendants of the given set.
    async fn descendants(&self, set: NameSet) -> Result<NameSet>;

    /// Calculates `roots` that are reachable from `heads` without going
    /// through other `roots`. For example, given the following graph:
    ///
    /// ```plain,ignore
    ///   F
    ///   |\
    ///   C E
    ///   | |
    ///   B D
    ///   |/
    ///   A
    /// ```
    ///
    /// `reachable_roots(roots=[A, B, C], heads=[F])` returns `[A, C]`.
    /// `B` is not included because it cannot be reached without going
    /// through another root `C` from `F`. `A` is included because it
    /// can be reached via `F -> E -> D -> A` that does not go through
    /// other roots.
    ///
    /// The can be calculated as
    /// `roots & (heads | parents(only(heads, roots & ancestors(heads))))`.
    /// Actual implementation might have faster paths.
    ///
    /// The `roots & ancestors(heads)` portion filters out bogus roots for
    /// compatibility, if the callsite does not provide bogus roots, it
    /// could be simplified to just `roots`.
    async fn reachable_roots(&self, roots: NameSet, heads: NameSet) -> Result<NameSet> {
        default_impl::reachable_roots(self, roots, heads).await
    }

    /// Get a snapshot of the current graph that can operate separately.
    ///
    /// This makes it easier to fight with borrowck.
    fn dag_snapshot(&self) -> Result<Arc<dyn DagAlgorithm + Send + Sync>>;
}

#[async_trait::async_trait]
pub trait Parents: Send + Sync {
    async fn parent_names(&self, name: VertexName) -> Result<Vec<VertexName>>;
}

#[async_trait::async_trait]
impl Parents for Arc<dyn DagAlgorithm + Send + Sync> {
    async fn parent_names(&self, name: VertexName) -> Result<Vec<VertexName>> {
        DagAlgorithm::parent_names(self, name).await
    }
}

#[async_trait::async_trait]
impl<'a> Parents for Box<dyn Fn(VertexName) -> Result<Vec<VertexName>> + Send + Sync + 'a> {
    async fn parent_names(&self, name: VertexName) -> Result<Vec<VertexName>> {
        (self)(name)
    }
}

#[async_trait::async_trait]
impl Parents for std::collections::HashMap<VertexName, Vec<VertexName>> {
    async fn parent_names(&self, name: VertexName) -> Result<Vec<VertexName>> {
        match self.get(&name) {
            Some(v) => Ok(v.clone()),
            None => name.not_found(),
        }
    }
}

/// Add vertexes recursively to the DAG.
#[async_trait::async_trait]
pub trait DagAddHeads {
    /// Add vertexes and their ancestors to the DAG. This does not persistent
    /// changes to disk.
    async fn add_heads(&mut self, parents: &dyn Parents, heads: &[VertexName]) -> Result<()>;
}

/// Import a generated `CloneData` object into the DAG.
pub trait DagImportCloneData {
    /// Updates the DAG using a `CloneData` object.
    fn import_clone_data(&mut self, clone_data: CloneData<VertexName>) -> Result<()>;
}

/// Persistent the DAG on disk.
#[async_trait::async_trait]
pub trait DagPersistent {
    /// Write in-memory DAG to disk. This might also pick up changes to
    /// the DAG by other processes.
    async fn flush(&mut self, master_heads: &[VertexName]) -> Result<()>;

    /// A faster path for add_heads, followed by flush.
    async fn add_heads_and_flush(
        &mut self,
        parent_names_func: &dyn Parents,
        master_names: &[VertexName],
        non_master_names: &[VertexName],
    ) -> Result<()>;

    /// Import from another (potentially large) DAG. Write to disk immediately.
    async fn import_and_flush(
        &mut self,
        dag: &dyn DagAlgorithm,
        master_heads: NameSet,
    ) -> Result<()> {
        let heads = dag.heads(dag.all().await?).await?;
        let non_master_heads = heads - master_heads.clone();
        let master_heads: Vec<VertexName> = master_heads.iter()?.collect::<Result<Vec<_>>>()?;
        let non_master_heads: Vec<VertexName> =
            non_master_heads.iter()?.collect::<Result<Vec<_>>>()?;
        self.add_heads_and_flush(&dag.dag_snapshot()?, &master_heads, &non_master_heads)
            .await
    }
}

/// Import ASCII graph to DAG.
pub trait ImportAscii {
    /// Import vertexes described in an ASCII graph.
    /// `heads` optionally specifies the order of heads to insert.
    /// Useful for testing. Panic if the input is invalid.
    fn import_ascii_with_heads(
        &mut self,
        text: &str,
        heads: Option<&[impl AsRef<str>]>,
    ) -> Result<()>;

    /// Import vertexes described in an ASCII graph.
    fn import_ascii(&mut self, text: &str) -> Result<()> {
        self.import_ascii_with_heads(text, <Option<&[&str]>>::None)
    }
}

/// Lookup vertexes by prefixes.
#[async_trait::async_trait]
pub trait PrefixLookup {
    /// Lookup vertexes by hex prefix.
    async fn vertexes_by_hex_prefix(
        &self,
        hex_prefix: &[u8],
        limit: usize,
    ) -> Result<Vec<VertexName>>;
}

/// Convert between `Vertex` and `Id`.
#[async_trait::async_trait]
pub trait IdConvert: PrefixLookup + Sync {
    async fn vertex_id(&self, name: VertexName) -> Result<Id>;
    async fn vertex_id_with_max_group(
        &self,
        name: &VertexName,
        max_group: Group,
    ) -> Result<Option<Id>>;
    async fn vertex_name(&self, id: Id) -> Result<VertexName>;
    async fn contains_vertex_name(&self, name: &VertexName) -> Result<bool>;
    async fn vertex_id_optional(&self, name: &VertexName) -> Result<Option<Id>> {
        self.vertex_id_with_max_group(name, Group::NON_MASTER).await
    }

    /// Identity of the map. If two maps have a same id, they are considered compatible.
    fn map_id(&self) -> &str;
}

impl<T> ImportAscii for T
where
    T: DagAddHeads,
{
    fn import_ascii_with_heads(
        &mut self,
        text: &str,
        heads: Option<&[impl AsRef<str>]>,
    ) -> Result<()> {
        let parents = drawdag::parse(&text);
        let heads: Vec<_> = match heads {
            Some(heads) => heads
                .iter()
                .map(|s| VertexName::copy_from(s.as_ref().as_bytes()))
                .collect(),
            None => {
                let mut heads: Vec<_> = parents
                    .keys()
                    .map(|s| VertexName::copy_from(s.as_bytes()))
                    .collect();
                heads.sort();
                heads
            }
        };

        let v = |s: String| VertexName::copy_from(s.as_bytes());
        let parents: std::collections::HashMap<VertexName, Vec<VertexName>> = parents
            .into_iter()
            .map(|(k, vs)| (v(k), vs.into_iter().map(v).collect()))
            .collect();
        nonblocking::non_blocking_result(self.add_heads(&parents, &heads[..]))?;
        Ok(())
    }
}

#[async_trait::async_trait]
pub trait ToIdSet {
    /// Converts [`NameSet`] to [`SpanSet`].
    async fn to_id_set(&self, set: &NameSet) -> Result<IdSet>;
}

pub trait ToSet {
    /// Converts [`SpanSet`] to [`NameSet`].
    fn to_set(&self, set: &IdSet) -> Result<NameSet>;
}

pub trait IdMapSnapshot {
    /// Get a snapshot of IdMap.
    fn id_map_snapshot(&self) -> Result<Arc<dyn IdConvert + Send + Sync>>;
}

/// Describes how to persist state to disk.
pub trait Persist {
    /// Return type of `lock()`.
    type Lock: Send + Sync;

    /// Obtain an exclusive lock for writing.
    /// This should prevent other writers.
    fn lock(&mut self) -> Result<Self::Lock>;

    /// Reload from the source of truth. Drop pending changes.
    ///
    /// This requires a lock and is usually called before `persist()`.
    fn reload(&mut self, _lock: &Self::Lock) -> Result<()>;

    /// Write pending changes to the source of truth.
    ///
    /// This requires a lock.
    fn persist(&mut self, _lock: &Self::Lock) -> Result<()>;

    /// Return a [`Locked`] instance that provides race-free filesytem read and
    /// write access by taking an exclusive lock.
    fn prepare_filesystem_sync(&mut self) -> Result<Locked<Self>>
    where
        Self: Sized,
    {
        let lock = self.lock()?;
        self.reload(&lock)?;
        Ok(Locked { inner: self, lock })
    }
}

/// Address that can be used to open things.
///
/// The address type decides the return type of `open`.
pub trait Open: Clone {
    type OpenTarget;

    fn open(&self) -> Result<Self::OpenTarget>;
}

/// Fallible clone.
pub trait TryClone {
    fn try_clone(&self) -> Result<Self>
    where
        Self: Sized;
}

impl<T: Clone> TryClone for T {
    fn try_clone(&self) -> Result<Self> {
        Ok(self.clone())
    }
}

#[async_trait::async_trait]
impl<T: IdConvert + IdMapSnapshot> ToIdSet for T {
    /// Converts [`NameSet`] to [`IdSet`].
    async fn to_id_set(&self, set: &NameSet) -> Result<IdSet> {
        // Fast path: extract IdSet from IdStaticSet.
        if let Some(set) = set.as_any().downcast_ref::<IdStaticSet>() {
            let snapshot = self.id_map_snapshot()?;
            if set.hints().is_id_map_compatible(snapshot) {
                return Ok(set.spans.clone());
            }
        }

        // Convert IdLazySet to IdStaticSet. Bypass hash lookups.
        if let Some(set) = set.as_any().downcast_ref::<IdLazySet>() {
            let snapshot = self.id_map_snapshot()?;
            if set.hints().is_id_map_compatible(snapshot) {
                let set: IdStaticSet = set.to_static()?;
                return Ok(set.spans);
            }
        }

        // Slow path: iterate through the set and convert it to a non-lazy
        // IdSet. Does not bypass hash lookups.
        let mut spans = IdSet::empty();
        for name in set.iter()? {
            let name = name?;
            let id = self.vertex_id(name).await?;
            spans.push(id);
        }
        Ok(spans)
    }
}

impl IdMapSnapshot for Arc<dyn IdConvert + Send + Sync> {
    fn id_map_snapshot(&self) -> Result<Arc<dyn IdConvert + Send + Sync>> {
        Ok(self.clone())
    }
}

impl<T: IdMapSnapshot + DagAlgorithm> ToSet for T {
    /// Converts [`SpanSet`] to [`NameSet`].
    fn to_set(&self, set: &IdSet) -> Result<NameSet> {
        NameSet::from_spans_dag(set.clone(), self)
    }
}
