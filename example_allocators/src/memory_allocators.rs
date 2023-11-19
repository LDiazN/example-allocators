use std::mem::MaybeUninit;
/// These are memory allocators that allocate free memory in the same
/// way generational indices allocate new indices.

use std::{alloc, ops::DerefMut};
use std::ops::Deref;

#[derive(Debug, PartialEq, Default, Clone)]
pub struct GenerationalIndex
{
    index : usize,
    generation : u32
}

pub struct GenerationalPointerEntry<T>
{
    generation: u32,
    ptr: *mut T
}

/// This is a handle-based allocators.
/// 
/// Users will get a handle that they have to query with this struct
/// to get the actual reference to the thing they want.
pub struct GenerationalPointersArray<T>
{
    entries : Vec<GenerationalPointerEntry<T>>,
    free: Vec<usize>
}

impl<T> GenerationalPointersArray<T>
{
    pub fn new() -> Self
    {
        GenerationalPointersArray
        {
            entries: vec![],
            free: vec![]
        }
    }

    pub fn allocate(&mut self) -> GenerationalIndex
    {
        if self.free.is_empty()
        {
            let new_entry = self._allocate_entry();
            let new_entry_index = self.entries.len();
            self.entries.push(new_entry);

            return GenerationalIndex{index: new_entry_index, generation: 0};
        }

        let next_free = self.free.pop().unwrap();
        let entry : &GenerationalPointerEntry<T> = &self.entries[next_free];

        let gen_index = GenerationalIndex{
            index : next_free,
            generation : entry.generation
        };

        return gen_index;
    }

    /// Called internally when we have to create a new entry instead of 
    /// reusing an old one
    fn _allocate_entry(&mut self) -> GenerationalPointerEntry<T>
    {
        unsafe
        {
            let item_mem = alloc::alloc(alloc::Layout::new::<T>()).cast::<T>();
            GenerationalPointerEntry { generation: 0, ptr: item_mem }
        }
    }

    #[inline(always)]
    pub fn is_live(&self, index: &GenerationalIndex) -> bool
    {
        return index.generation == self.entries[index.index].generation;
    }

    pub fn get(&self, index: &GenerationalIndex) -> Option<&mut T>
    {
        if !self.is_live(index)
        {
            return None;
        }

        let entry : &GenerationalPointerEntry<T> = &self.entries[index.index];
        return unsafe { Some(entry.ptr.as_mut().unwrap_unchecked())}
    }

    pub fn free(&mut self, index: &GenerationalIndex)
    {
        if self.is_live(index)
        {
            panic!("Trying to free already unused index");
        }

        let index = index.index;
        self.free.push(index);
        let entry : &mut GenerationalPointerEntry<T> = &mut self.entries[index];
        entry.generation += 1;
    }
}

// The following version is similar to the one before but we use pointers as the handle to 
// simplify and optimize access

pub type Generation = u32;

#[derive(Debug, Default)]
pub struct EntryHeader
{
    generation: Generation,
}

#[derive(Debug)]
#[repr(C)]
pub struct EntityEntry<T>
{
    header: EntryHeader,
    item: MaybeUninit<T>
}

#[derive(Debug, Clone, Copy)]
pub struct EntityPtr<T>
{
    ptr : *mut T,
    generation: Generation
}

pub struct EntityAllocator<T> 
{
    free: Vec<*mut T>,
    entries: Vec<Box<EntityEntry<T>>>
}


impl<T> EntityAllocator<T>
    where T : Default
{
    pub fn new() -> Self
    {
        EntityAllocator
        {
            entries: vec![],
            free: vec![]
        }
    }

    pub fn allocate(&mut self, init_fn : fn(&mut T)) -> EntityPtr<T>
    {
        if self.free.is_empty()
        {
            // Allocate a new entry
            let mut mem = MaybeUninit::<T>::uninit();
            unsafe { mem.as_mut_ptr().write(T::default()) };

            let mut new_entry = Box::new(
                EntityEntry{
                    header: EntryHeader::default(), 
                    item: mem
                }
            );

            // Pointer to return
            let t_ptr = new_entry.item.as_mut_ptr();

            // Initialize new entry:
            unsafe {
                (init_fn)(new_entry.item.as_mut_ptr().as_mut().unwrap());   
            }
            
            self.entries.push(new_entry);

            // Create pointer:
            return EntityPtr{ptr: t_ptr, generation: 0};
        }

        let entity_ptr = self.free.pop().unwrap();
        let entry = unsafe {EntityEntry::from_ptr(entity_ptr)};
        unsafe {
            (init_fn)(entry.item.as_mut_ptr().as_mut().unwrap());
        }

        return EntityPtr{ptr: entity_ptr, generation: entry.header.generation};
    }

    pub fn free(&mut self, entity_ptr: &EntityPtr<T>)
    {
        if !entity_ptr.is_live()
        {
            panic!("Trying to free already unused index");
        }

        let entry  = unsafe {EntityEntry::from_ptr(entity_ptr.ptr)};
        entry.header.generation += 1;
        unsafe { entry.item.assume_init_drop() };

        self.free.push(entity_ptr.ptr);
    }
}

impl<T> Deref for EntityPtr<T> {
    
    type Target = T;

    fn deref(&self) -> &Self::Target {
        assert!(self.is_live(), "Trying to deref invalid entity ptr");
        unsafe
        {
            self.ptr.as_ref().unwrap()
        }
    }
}

impl<T> DerefMut for EntityPtr<T>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        assert!(self.is_live(), "Trying to deref invalid entity ptr");
        unsafe
        {
            self.ptr.as_mut().unwrap()
        }
    }
}

impl<'a, T> EntityEntry<T>
{
    unsafe fn from_ptr(ptr: *mut T) -> &'a mut Self
    {
        ptr
            .cast::<u8>()
            .sub(std::mem::size_of::<EntityEntry<T>>() - std::mem::size_of::<T>())
            .cast::<EntityEntry<T>>()
            .as_mut()
            .unwrap()
    }
}

impl<T> EntityPtr<T>
{
    pub fn is_live(&self) -> bool
    {
        let entry = unsafe {EntityEntry::from_ptr(self.ptr)};

        return entry.header.generation == self.generation;
    }
}