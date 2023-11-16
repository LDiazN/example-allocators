/// These are memory allocators that allocate free memory in the same
/// way generational indices allocate new indices.

use std::alloc;


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