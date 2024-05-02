#[allow(unused)]
use std::borrow::Borrow;
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};

pub type Generation = u32;

/// This is a pointer-based allocator.
///
/// The pointer will have a reference to an object allocated within the allocator
#[derive(Default)]
pub struct Allocator<T> {
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

impl<T> Allocator<T> {
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