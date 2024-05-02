/// These are memory allocators that allocate free memory in the same
/// way generational indices allocate new indices.
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ops::DerefMut;
use std::cell::RefCell;

/// Default Index type for handle based implementations
#[derive(Debug, PartialEq, Default, Clone)]
pub struct GenerationalIndex {
    index: usize,
    generation: Generation,
}
pub type Generation = u32;

/// This is a handle-based allocators.
///
/// Users will get a handle that they have to query with this struct
/// to get the actual reference to the thing they want.
#[derive(Default)]
pub struct GIABoxUninit<T> {
    entries: Vec<GIABoxUninitEntry<T>>,
    free: Vec<usize>,
}

pub struct GIABoxUninitEntry<T> {
    generation: Generation,
    ptr: Box<MaybeUninit<RefCell<T>>>,
}

impl<T> GIABoxUninit<T> {
    pub fn new(&mut self, element: T) -> GenerationalIndex {
        if self.free.is_empty() {
            // Construct a new entry
            let mut new_entry = GIABoxUninitEntry {
                generation: 0,
                ptr: Box::new(MaybeUninit::<RefCell<T>>::uninit()),
            };
            let new_entry_index = self.entries.len();

            // Initialize it since it will be retrieved from this function
            new_entry.ptr.write(RefCell::new(element));

            // Add it to the current list of entries
            self.entries.push(new_entry);

            return GenerationalIndex {
                index: new_entry_index,
                generation: 0,
            };
        }

        let next_free = self.free.pop().unwrap();
        let entry = &mut self.entries[next_free];

        // Initialize entry, don't return uninitialized memory
        entry.ptr.write(RefCell::new(element));

        return GenerationalIndex {
            index: next_free,
            generation: entry.generation,
        };
    }

    #[inline(always)]
    pub fn is_live(&self, index: &GenerationalIndex) -> bool {
        return index.generation == self.entries[index.index].generation;
    }

    pub fn get(&self, index: &GenerationalIndex) -> Option<&RefCell<T>> {
        if !self.is_live(index) {
            return None;
        }

        return unsafe {
            Some(self.entries[index.index].ptr.assume_init_ref())
        };
    }

    pub fn free(&mut self, index: &GenerationalIndex) {
        if !self.is_live(index) {
            panic!("Trying to free already unused index");
        }

        let index = index.index;
        self.free.push(index);
        let entry: &mut GIABoxUninitEntry<T> = &mut self.entries[index];
        entry.generation += 1;
        unsafe {
            entry.ptr.assume_init_drop();
        }
    }
}

// The following version is similar to the one before but we use pointers as the handle to
// simplify and optimize access
#[derive(Debug, Default)]
pub struct EntityAllocator<T> {
    free: Vec<*mut T>,
    entries: Vec<Box<EntityEntry<T>>>,
}

#[derive(Debug)]
#[repr(C)]
pub struct EntityEntry<T> {
    header: EntryHeader,
    item: MaybeUninit<T>,
}

#[derive(Debug, Default)]
pub struct EntryHeader {
    generation: Generation,
}

#[derive(Debug, Clone, Copy)]
pub struct EntityPtr<T> {
    ptr: *mut T,
    generation: Generation,
}

impl<T> EntityAllocator<T>
where
    T: Default,
{
    pub fn allocate(&mut self, init_fn: impl Fn(&mut T)) -> EntityPtr<T> {
        if self.free.is_empty() {
            // Allocate a new entry
            let mut mem = MaybeUninit::<T>::uninit();
            mem.write(T::default());

            let mut new_entry = Box::new(EntityEntry {
                header: EntryHeader::default(),
                item: mem,
            });

            // Pointer to return
            let t_ptr = new_entry.item.as_mut_ptr();

            // Initialize new entry:
            unsafe {
                (init_fn)(new_entry.item.as_mut_ptr().as_mut().unwrap());
            }

            self.entries.push(new_entry);

            // Create pointer:
            return EntityPtr {
                ptr: t_ptr,
                generation: 0,
            };
        }

        let entity_ptr = self.free.pop().unwrap();
        let entry = unsafe { EntityEntry::from_ptr(entity_ptr) };
        unsafe {
            (init_fn)(entry.item.as_mut_ptr().as_mut().unwrap());
        }

        return EntityPtr {
            ptr: entity_ptr,
            generation: entry.header.generation,
        };
    }

    pub fn free(&mut self, entity_ptr: &EntityPtr<T>) {
        if !entity_ptr.is_live() {
            panic!("Trying to free already unused index");
        }

        let entry = unsafe { EntityEntry::from_ptr(entity_ptr.ptr) };
        entry.header.generation += 1;
        unsafe { entry.item.assume_init_drop() };

        self.free.push(entity_ptr.ptr);
    }
}

impl<T> Deref for EntityPtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        debug_assert!(self.is_live(), "Trying to deref invalid entity ptr");
        unsafe { self.ptr.as_ref().unwrap() }
    }
}

impl<T> DerefMut for EntityPtr<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        debug_assert!(self.is_live(), "Trying to deref invalid entity ptr");
        unsafe { self.ptr.as_mut().unwrap() }
    }
}

impl<'a, T> EntityEntry<T> {
    unsafe fn from_ptr(ptr: *mut T) -> &'a mut Self {
        ptr.cast::<u8>()
            .sub(std::mem::size_of::<EntityEntry<T>>() - std::mem::size_of::<T>())
            .cast::<EntityEntry<T>>()
            .as_mut()
            .unwrap()
    }
}

impl<T> EntityPtr<T> {
    pub fn is_live(&self) -> bool {
        let entry = unsafe { EntityEntry::from_ptr(self.ptr) };

        return entry.header.generation == self.generation;
    }
}

// The following example is a handle based implementation
// with in-place memory segments, meaning that all entities will be contiguous in memory,
// which should speed up access for multiple entities, but might be slower when allocating new entities
#[derive(Debug, Default)]
pub struct InPlaceAllocator<T>
{
    entries: Vec<InPlaceAllocEntry<T>>,
    free: Vec<usize>,
}

#[derive(Debug)]
struct InPlaceAllocEntry<T> {
    // Note that since MaybeUninit has transparent layout, this is the same as having an actual T
    // inside the struct. Having an array of these is the same as having an array of T,
    // making it in place
    value: RefCell<MaybeUninit<T>>,
    generation: Generation,
}

impl<T> InPlaceAllocator<T>
{
    pub fn new(&mut self, element : T) -> GenerationalIndex {
        if self.free.is_empty() {
            // Construct a new entry
           let mut new_entry = InPlaceAllocEntry {
                value: RefCell::new(MaybeUninit::<T>::uninit()),
                generation: 0,
            };
            let new_entry_index = self.entries.len();

            // Initialize it since it will be retrieved from this function
            new_entry.value.borrow_mut().write(element);

            // Add it to the current list of entries
            self.entries.push(new_entry);

            return GenerationalIndex {
                index: new_entry_index,
                generation: 0,
            };
        }

        let next_free = self.free.pop().unwrap();
        let entry = &mut self.entries[next_free];

        // Initialize entry, don't return uninitialized memory
        entry.value.borrow_mut().write(element);

        return GenerationalIndex {
            index: next_free,
            generation: entry.generation,
        };
    }

    #[inline(always)]
    pub fn is_live(&self, index: &GenerationalIndex) -> bool {
        return index.generation == self.entries[index.index].generation;
    }

    pub fn get(&self, index: &GenerationalIndex) -> &mut T {
        debug_assert!(
            self.is_live(index),
            "Trying to retrieve uninitialized memory"
        );

        let entry = &self.entries[index.index];
        return unsafe { entry.value.borrow_mut().as_mut_ptr().as_mut().unwrap() };
    }

    pub fn free(&mut self, index: &GenerationalIndex) {
        if !self.is_live(index) {
            panic!("Trying to free already unused index");
        }

        let index = index.index;
        self.free.push(index);
        let entry = &mut self.entries[index];
        entry.generation += 1;
        unsafe {
            entry.value.borrow_mut().assume_init_drop();
        }
    }
}
