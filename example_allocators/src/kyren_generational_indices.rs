/// This is a simple generational indices implementation using Katherine West' draft implementation
/// presented here: https://kyren.github.io/2018/09/14/rustconf-talk.html
/// 
/// This is the base implementation I will be testing my allocators with.
use std::collections::VecDeque;

#[derive(Debug, PartialEq, Default)]
/// This is the simplest implementation, this struct will tell you which index
/// to use next, but the actual objects should be managed by yourself. 
pub struct GenerationalIndices
{
    indices : Vec<u32>, // Generations. Indices are specified by the array position
    pub free : VecDeque<usize>
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct GenerationalIndex
{
    index : usize,
    generation : u32
}

impl GenerationalIndices
{
    pub fn new(&mut self) -> GenerationalIndex
    {
        if self.free.is_empty()
        {
            let next_index = self.indices.len();
            self.indices.push(0);

            return GenerationalIndex{index: next_index, generation: 0};
        }

        let index = self.free.pop_front().unwrap();
        let generation = self.indices[index];

        GenerationalIndex { index, generation}
    }

    #[inline(always)]
    pub fn is_live(&self, index:  &GenerationalIndex) -> bool
    {
        index.generation == self.indices[index.index]
    }

    pub fn free(&mut self, index:&GenerationalIndex)
    {
        if ! self.is_live(&index)
        {
            return; // Report an error or something
        }

        self.free.push_back(index.index);
        self.indices[index.index] += 1;
    }
}

impl GenerationalIndex
{
    #[inline(always)]
    pub fn get_generation(&self) -> u32
    {
        self.generation
    }

    #[inline(always)]
    pub fn get_index(&self) -> usize
    {
        self.index
    }
}

// -- < Versions with the actual storage > ------------------------
#[derive(Debug, Default)]
pub struct GenerationalArrayEntry<T>
{
    item : Option<T>,
    generation : u32
}

/// This version also implements the storage for the thing being identified. 
/// 
/// It has some drawbacks: 
/// 
///  * You might have to resize an array with too many elements with possibly large storage
///  * You have to construct objects in the stack and then copy the entire content into the internal array
///  * You might end up with a lot of unused unrecoverable space after a lot of allocations
#[derive(Debug, Default)]
pub struct GenerationalIndexArray<T>
{
    elements : Vec<GenerationalArrayEntry<T>>,
    free: Vec<usize>
}

impl<T> GenerationalIndexArray<T>
{
    pub fn new(&mut self, element : T) -> GenerationalIndex
    {
        if self.free.is_empty()
        {
            let next_index = self.elements.len();
            let entry = GenerationalArrayEntry{generation: 0, item: Some(element)};
            self.elements.push(entry);

            return GenerationalIndex{index: next_index, generation: 0};
        }

        let index = self.free.pop().unwrap();
        let entry = &mut self.elements[index];
        entry.item = Some(element);

        GenerationalIndex {index, generation: entry.generation}
    }

    #[inline(always)]
    pub fn is_live(&self, index:  &GenerationalIndex) -> bool
    {
        index.get_generation() == self.elements[index.index].generation
    }

    pub fn free(&mut self, index:&GenerationalIndex)
    {
        if !self.is_live(&index)
        {
            panic!("Trying to free an already dead index");
        }

        self.free.push(index.index);
        self.elements[index.index].generation += 1;
        self.elements[index.index].item = None;
    }

    pub fn get(&self, index: &GenerationalIndex) -> Option<&T>
    {
        if !self.is_live(index)
        {
            return None;
        }

        return self.elements[index.get_index()].item.as_ref();
    }

    pub fn get_mut(&mut self, index: &GenerationalIndex) -> Option<&mut T>
    {
        if !self.is_live(index)
        {
            return None;
        }

        return self.elements[index.get_index()].item.as_mut();
    }
}

// Te previous implementation has some problems about references and pointers. So instead 
// we will store pointers instead of the entire thing we are allocating.