use std::borrow::Borrow;
use std::mem::MaybeUninit;
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};

pub type Generation = u32;

/// This is a handle-based allocators.
///
/// Users will get a handle that they have to query with this struct
/// to get the actual reference to the thing they want.
#[derive(Default)]
pub struct Allocator<T> {
    entries: Vec<Box<Entry<T>>>,
    free: Vec<*mut Entry<T>>,
}

pub struct Entry<T> {
    generation: Generation,
    ptr: MaybeUninit<T>
}

pub struct EntityPtr<T> {
    generation: Generation,
    ptr: *mut Entry<T>, // super unsafe raw pointer!
}

impl<T> Allocator<T> {
    pub fn new(&mut self, element: T) -> EntityPtr<T> {
        if self.free.is_empty() {
            // Construct a new entry
            let mut new_entry = Box::new(Entry {
                generation: 0,
                ptr: MaybeUninit::<T>::uninit(),
            });
            let new_entry_index = self.entries.len();

            // Initialize it since it will be retrieved from this function
            new_entry.ptr.write(element);


            // Add it to the current list of entries
            self.entries.push(new_entry);

            return EntityPtr{
                ptr: &mut *self.entries[new_entry_index] as  *mut Entry<T>,
                generation: 0,
            };
        }

        let next_free = self.free.pop().unwrap();

        // Initialize entry, don't return uninitialized memory
        unsafe{(*next_free).ptr.write(element)};

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
           (*ptr.ptr).ptr.assume_init_drop();
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
        return unsafe {(*self.ptr).ptr.assume_init_ref()}
    }
}

impl <T> DerefMut for EntityPtr<T> {

    fn deref_mut(&mut self) -> &mut Self::Target {
        debug_assert!(self.is_live(), "Trying to deref free pointer");
        return unsafe {(*self.ptr).ptr.assume_init_mut()}
    }
}