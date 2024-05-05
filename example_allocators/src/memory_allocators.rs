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
/// This is a pointer-based allocator.
///
/// The pointer will have a reference to an object allocated within the allocator
#[derive(Default)]
pub struct BoxAllocator<T> {
    entries: Vec<Box<Entry<T>>>,
    free: Vec<*mut Entry<T>>,
}

pub struct Entry<T> {
    generation: Generation,
    value: MaybeUninit<T>
}

// To keep this implementation safe, you should not allow the user to construct 
// an EntityPtr by themselves, always ask the allocator to give you a new one
pub struct EntityPtr<T> {
    generation: Generation,
    ptr: *mut Entry<T>, // super unsafe raw pointer!
}

impl<T> BoxAllocator<T> {
    pub fn new(&mut self, element: T) -> EntityPtr<T> {
        if self.free.is_empty() {
            // Construct a new entry
            let mut new_entry = Box::new(Entry {
                generation: 0,
                value: MaybeUninit::<T>::uninit(),
            });
            let new_entry_index = self.entries.len();

            // Initialize it since it will be retrieved from this function
            new_entry.value.write(element);


            // Add it to the current list of entries
            self.entries.push(new_entry);

            return EntityPtr{
                ptr: &mut *self.entries[new_entry_index] as  *mut Entry<T>,
                generation: 0,
            };
        }

        let next_free = self.free.pop().unwrap();

        // Initialize entry, don't return uninitialized memory
        unsafe{(*next_free).value.write(element)};

        let generation = unsafe {
            (*next_free).generation
        };

        return EntityPtr{
            ptr: next_free,
            generation: generation
        }
    }

    pub fn free(&mut self, ptr: &EntityPtr<T>) {

        debug_assert!(ptr.is_live(), "Trying to double-free a pointer");
        self.free.push(ptr.ptr);
        unsafe {
           (*ptr.ptr).generation += 1;
           (*ptr.ptr).value.assume_init_drop();
        }
    }
}

impl<T> EntityPtr<T> {
    #[inline(always)]
    pub fn is_live(&self) -> bool {
        return self.generation == unsafe {(*self.ptr).generation}
    }
}

impl <T> Deref for EntityPtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        debug_assert!(self.is_live(), "Trying to deref free pointer");
        return unsafe {(*self.ptr).value.assume_init_ref()}
    }
}

impl <T> DerefMut for EntityPtr<T> {

    fn deref_mut(&mut self) -> &mut Self::Target {
        debug_assert!(self.is_live(), "Trying to deref free pointer");
        return unsafe {(*self.ptr).value.assume_init_mut()}
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
