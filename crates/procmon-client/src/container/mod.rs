use std::{
    mem::{swap, MaybeUninit},
    ops::Range,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, RwLock,
    },
};

/// A chunked vector for append‐only storage with thread‐safe access.
/// Internally, the vector is backed by one or more fixed-size chunks
/// allocated using `MaybeUninit<T>`.
pub struct ChunkVecRaw<T> {
    /// Fixed capacity of each allocated chunk; always a power of two.
    default_size: usize,
    /// How many bits to shift an index to obtain the container (chunk) index.
    container_shift: usize,
    /// Bit mask used to obtain the offset within a container.
    element_mask: usize,
    /// The inner storage, protected by an `RwLock`.
    /// The tuple consists of:
    /// - a vector of allocated boxed slices (chunks) holding `MaybeUninit<T>`.
    /// - the current total capacity (in elements) allocated.
    inner: RwLock<(Vec<Box<[MaybeUninit<T>]>>, usize)>,
}

impl<T> ChunkVecRaw<T> {
    /// Creates an empty `ChunkVecRaw` with the given inner size (will be rounded to a power of two).
    pub fn empty(inner_size: usize) -> Self {
        let default_size = inner_size.next_power_of_two();
        Self {
            default_size,
            container_shift: default_size.trailing_zeros() as usize,
            element_mask: default_size - 1,
            inner: RwLock::new((Vec::new(), default_size)),
        }
    }

    /// Creates a new `ChunkVecRaw` pre-allocated with one chunk.
    pub fn new(inner_size: usize) -> Self {
        let default_size = inner_size.next_power_of_two();
        Self {
            default_size,
            container_shift: default_size.trailing_zeros() as usize,
            element_mask: default_size - 1,
            inner: RwLock::new((vec![Self::alloc_container(default_size)], default_size)),
        }
    }

    /// Helper function to allocate a new container/chunk.
    #[inline(always)]
    fn alloc_container(capacity: usize) -> Box<[MaybeUninit<T>]> {
        // Safety: Box::new_uninit_slice() creates a boxed slice of uninitialized memory.
        Box::new_uninit_slice(capacity)
    }

    /// Computes the container index (i.e. which chunk) for the given element index.
    #[inline]
    pub fn container_id(&self, index: usize) -> usize {
        index >> self.container_shift
    }

    /// Computes the offset within a container for the given element index.
    #[inline]
    pub fn offset_in_container(&self, index: usize) -> usize {
        index & self.element_mask
    }

    /// Ensures that the storage has enough capacity to hold an element at `offset + len - 1`.
    pub fn resize_if_necessary(&self, offset: usize, len: usize) {
        {
            // Optimistic check with shared lock.
            let read_guard = self.inner.read().unwrap();
            if offset + len <= read_guard.1 {
                return;
            }
            // Drop the guard to avoid deadlocking when acquiring write lock.
        }

        // Acquire write lock only if needed.
        let mut writer = self.inner.write().unwrap();
        if offset + len <= writer.1 {
            // Another thread may have already resized.
            return;
        }
        // Add one new container.
        writer.0.push(Self::alloc_container(self.default_size));
        writer.1 += self.default_size;
    }

    /// Returns an immutable reference to the element at the given index.
    /// # Safety
    /// The caller must ensure the index is initialized.
    pub fn get<'a>(&'a self, index: usize) -> &'a MaybeUninit<T> {
        let container_idx = index >> self.container_shift;
        let offset = index & self.element_mask;
        &self.get_container_slice(container_idx)[offset]
    }

    /// Returns a slice for the given container (chunk) as immutable.
    pub fn get_container_slice<'a>(&'a self, container: usize) -> &'a [MaybeUninit<T>] {
        // We lock for a short time to retrieve the pointer; the container's allocation is stable.
        let inner = self.inner.read().unwrap().0[container].as_ptr();
        // Safety: The memory pointed to by the Box remains valid, and we know the slice length.
        unsafe { std::slice::from_raw_parts(inner as *const MaybeUninit<T>, self.default_size) }
    }

    /// Returns a slice for the given container (chunk) as mutable.
    pub fn get_container_slice_mut<'a>(&'a self, container: usize) -> &'a mut [MaybeUninit<T>] {
        let inner = self.inner.read().unwrap().0[container].as_ptr();
        // Safety: As above, we are allowed to create a mutable slice over the container.
        unsafe { std::slice::from_raw_parts_mut(inner as *mut MaybeUninit<T>, self.default_size) }
    }

    /// Appends a new container, expanding total capacity.
    pub fn push_container(&self) {
        let mut writer = self.inner.write().unwrap();
        writer.0.push(Self::alloc_container(self.default_size));
        writer.1 += self.default_size;
    }

    /// Obtains an immutable range from the vector.
    /// The range is defined by a start index and an end index.
    pub fn get_range<'a>(&'a self, range: Range<usize>) -> ChunkedRange<'a, T> {
        let count = range.end - range.start;
        ChunkedRange {
            owner: self,
            start_container: self.container_id(range.start),
            start_container_offset: self.offset_in_container(range.start),
            size: count,
        }
    }

    /// Obtains a mutable range from the vector.
    pub fn get_range_mut<'a>(&'a self, range: Range<usize>) -> ChunkedRangeMut<'a, T> {
        let count = range.end - range.start;
        ChunkedRangeMut {
            owner: self,
            start_container: self.container_id(range.start),
            start_container_offset: self.offset_in_container(range.start),
            size: count,
        }
    }

    /// Drops (and thus runs the destructor for) elements in order.
    /// Should be called on drop to cleanup initialized elements.
    /// # Safety
    /// The caller must ensure that exactly `count` elements were initialized.
    pub unsafe fn drop_elements(&self, mut count: usize) {
        let mut containers = Vec::new();
        {
            // Take ownership of the chunk vector so that no one else uses it.
            let mut write_guard = self.inner.write().unwrap();
            swap(&mut containers, &mut write_guard.0);
        }
        for mut container in containers {
            if count >= self.default_size {
                // Safety: Assume all elements in this container were initialized.
                container.assume_init();
                count -= self.default_size;
            } else {
                // Safety: Only the first `count` items are initialized.
                (&mut container[..count]).assume_init_drop();
                break;
            }
        }
    }

    /// Searches for an element matching the predicate `f` in reverse order up to `index`.
    /// Returns the absolute index of the found element, or `None` if not found.
    pub fn reverse_find<F>(&self, index: usize, mut f: F) -> Option<usize>
    where
        F: FnMut(&T) -> bool,
    {
        let last_container = self.container_id(index);
        let end_offset = self.offset_in_container(index);

        // Search in the last container from beginning up to end_offset.
        let first_slice = &self.get_container_slice(last_container)[..end_offset];
        if let Some(relative_idx) = first_slice.iter().rposition(|elem| {
            // Safety: Caller ensures this element is initialized.
            f(unsafe { elem.assume_init_ref() })
        }) {
            return Some((last_container << self.container_shift) | relative_idx);
        }
        // Then scan previous containers.
        for container in (0..last_container).rev() {
            let slice = self.get_container_slice(container);
            if let Some(relative_idx) = slice
                .iter()
                .rposition(|elem| f(unsafe { elem.assume_init_ref() }))
            {
                return Some((container << self.container_shift) | relative_idx);
            }
        }
        None
    }

    #[inline]
    pub fn default_size(&self) -> usize {
        self.default_size
    }
}

/// A helper structure for iterating immutably over a contiguous range of elements
/// possibly spanning multiple containers.
pub struct ChunkedRange<'a, T> {
    owner: &'a ChunkVecRaw<T>,
    start_container: usize,
    start_container_offset: usize,
    size: usize,
}

impl<'a, T> ChunkedRange<'a, T> {
    /// Iterates over each element in this chunked range.
    /// # Safety
    /// The callback receives a reference to `MaybeUninit<T>`,
    /// so it is the caller’s responsibility to only use initialized data.
    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(&MaybeUninit<T>),
    {
        let mut remaining = self.size;
        let mut container = self.start_container;
        let mut offset = self.start_container_offset;

        while remaining > 0 {
            let chunk = self.owner.get_container_slice(container);
            // Calculate how many elements to process in this container.
            let available = (self.owner.default_size - offset).min(remaining);
            for elem in &chunk[offset..offset + available] {
                f(elem);
            }
            remaining -= available;
            container += 1;
            offset = 0;
        }
    }
}

/// A helper structure for iterating mutably over a contiguous range of elements
/// possibly spanning multiple containers.
pub struct ChunkedRangeMut<'a, T> {
    owner: &'a ChunkVecRaw<T>,
    start_container: usize,
    start_container_offset: usize,
    size: usize,
}

impl<'a, T> ChunkedRangeMut<'a, T> {
    /// Iterates over each element in the mutable range.
    /// # Safety
    /// The callback receives a mutable reference to `MaybeUninit<T>`.
    /// The caller must ensure that accesses to the uninitialized memory are safe.
    pub fn for_each<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut MaybeUninit<T>),
    {
        let mut remaining = self.size;
        let mut container = self.start_container;
        let mut offset = self.start_container_offset;

        while remaining > 0 {
            let chunk = self.owner.get_container_slice_mut(container);
            let available = (self.owner.default_size - offset).min(remaining);
            for elem in &mut chunk[offset..offset + available] {
                f(elem);
            }
            remaining -= available;
            container += 1;
            offset = 0;
        }
    }
}

/// An append‐only chunked storage which utilizes the `ChunkVecRaw` for internal storage.
/// It supports concurrent writes (via atomics and a mutex for merging contiguous commits)
/// and read access to committed data.
pub struct ConcurrentChunkVec<T> {
    inner: ChunkVecRaw<T>,
    /// Number of elements that have been fully committed.
    commited_size: AtomicUsize,
    /// Total number of elements written so far.
    size: AtomicUsize,
    /// Holds commit-permits for non-contiguous writes.
    complete_permits: Mutex<Vec<WriteComplete>>,
}

unsafe impl<T: Send> Send for ConcurrentChunkVec<T> {}
unsafe impl<T: Send> Sync for ConcurrentChunkVec<T> {}

/// Represents the offset and size of a contiguous write.
struct WriteComplete {
    offset: usize,
    size: usize,
}

impl<T> ConcurrentChunkVec<T> {
    /// Creates a new chunked storage with the given inner (chunk) size.
    pub fn new(inner_size: usize) -> Self {
        Self {
            inner: ChunkVecRaw::new(inner_size),
            commited_size: AtomicUsize::new(0),
            size: AtomicUsize::new(0),
            complete_permits: Mutex::new(Vec::new()),
        }
    }

    /// Performs an in-place write of `size` elements by invoking the callback `writer_cb`
    /// for each element in the designated range.
    /// Returns `false` if the `size` exceeds the container capacity.
    pub fn acquire_write_inplace<W>(&self, size: usize, mut writer_cb: W) -> bool
    where
        W: FnMut(usize) -> T,
    {
        // Early exit if request is larger than a single container.
        if size > self.inner.default_size() {
            return false;
        }

        // Atomically fetch the offset at which to write.
        let offset = self.size.fetch_add(size, Ordering::AcqRel);
        // Ensure the underlying storage has enough capacity.
        self.inner.resize_if_necessary(offset, size);

        // Write in-place to the allocated range.
        let mut range = self.inner.get_range_mut(offset..(offset + size));
        let mut index = 0;
        range.for_each(|elem| {
            // Safety: Writing into uninitialized memory.
            elem.write(writer_cb(index));
            index += 1;
        });

        // Merge commit permits.
        let mut guard = self.complete_permits.lock().unwrap();
        let last_commited = self.commited_size.load(Ordering::SeqCst);

        // Insert in sorted order.
        let pos = guard
            .binary_search_by(|probe| probe.offset.cmp(&offset))
            .unwrap_or_else(|pos| pos);
        guard.insert(pos, WriteComplete { offset, size });

        // Merge contiguous commits starting from the earliest uncommitted offset.
        if last_commited != offset {
            return true;
        }
        let mut current_offset = offset;
        let mut last_index = 0;
        let mut sum = 0;
        for (i, wc) in guard.iter().enumerate() {
            if wc.offset != current_offset {
                break;
            }
            last_index = i;
            current_offset += wc.size;
            sum += wc.size;
        }
        // Remove merged entries.
        guard.drain(..=last_index);
        self.commited_size.fetch_add(sum, Ordering::Release);

        true
    }

    /// Provides an immutable chunked range for the committed data.
    /// Panics if the requested range is out-of-bounds.
    pub fn get_range<'a>(&'a self, range: Range<usize>) -> ConcurrentChunkVecRange<'a, T> {
        let commited = self.commited_size.load(Ordering::Acquire);
        if range.end > commited {
            panic!(
                "Out of range: requested {} but only {} committed",
                range.end, commited
            );
        }
        ConcurrentChunkVecRange(self.inner.get_range(range))
    }

    /// Provides a mutable chunked range for the committed data.
    /// Panics if the requested range is out-of-bounds.
    pub fn get_range_mut<'a>(&'a self, range: Range<usize>) -> ConcurrentChunkVecRangeMut<'a, T> {
        let commited = self.commited_size.load(Ordering::Acquire);
        if range.end > commited {
            panic!(
                "Out of range: requested {} but only {} committed",
                range.end, commited
            );
        }
        ConcurrentChunkVecRangeMut(self.inner.get_range_mut(range))
    }

    /// Retrieves a reference to a committed element, if it exists.
    pub fn get<'a>(&'a self, index: usize) -> Option<&'a T> {
        if index >= self.len() {
            return None;
        }
        // Safety: Caller is asking only for committed elements.
        Some(unsafe { self.inner.get(index).assume_init_ref() })
    }

    /// Returns the number of committed elements.
    pub fn len(&self) -> usize {
        self.commited_size.load(Ordering::Relaxed)
    }
}

/// A read-only range handle for data storage.
pub struct ConcurrentChunkVecRange<'a, T>(ChunkedRange<'a, T>);

impl<'a, T> ConcurrentChunkVecRange<'a, T> {
    /// Iterates over each element in this range.
    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(&T),
    {
        self.0.for_each(|elem| {
            // Safety: Each element in the range has been initialized.
            f(unsafe { elem.assume_init_ref() })
        });
    }
}

/// A mutable range handle for element storage.
pub struct ConcurrentChunkVecRangeMut<'a, T>(ChunkedRangeMut<'a, T>);

impl<'a, T> ConcurrentChunkVecRangeMut<'a, T> {
    /// Iterates mutably over each element in this range.
    pub fn for_each<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut T),
    {
        self.0.for_each(|elem| {
            // Safety: Each element in the range has been initialized.
            f(unsafe { elem.assume_init_mut() })
        });
    }
}

impl<T> Drop for ConcurrentChunkVec<T> {
    fn drop(&mut self) {
        // Safety: We drop exactly the number of committed elements.
        unsafe {
            self.inner
                .drop_elements(self.commited_size.load(Ordering::Acquire));
        }
    }
}
