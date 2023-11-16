/// This is a simple generational indices implementation using Katherine's West draft implementation
/// presented here: https://kyren.github.io/2018/09/14/rustconf-talk.html
/// 
/// This is the base implementation I will be testing my allocators with.

/// This is the simplest implementation, this struct will tell you which index
/// to use next, but the actual objects should be managed by yourself. 
#[derive(Debug, PartialEq, Default, Clone)]
pub struct GenerationalIndex
{
    index : usize,
    generation : u32
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

#[derive(Debug, PartialEq, Default)]
pub struct GenerationalIndices
{
    indices : Vec<u32>, // Generations. Indices are specified by the array position
    pub free : Vec<usize>
}

impl GenerationalIndices
{
    #[inline(always)]
    pub fn new() -> Self
    {
        GenerationalIndices { indices: vec![], free: vec![] }
    }

    pub fn get(&mut self) -> GenerationalIndex
    {
        if self.free.is_empty()
        {
            let next_index = self.indices.len();
            self.indices.push(0);

            return GenerationalIndex{index: next_index, generation: 0};
        }

        let index = self.free.pop().unwrap();
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

        self.free.push(index.index);
        self.indices[index.index] += 1;
    }
}

// -- < Versions with the actual storage > ------------------------
pub trait LifeCycle {
    fn destroy(&mut self);

    fn set_index(&mut self, index: GenerationalIndex);
}
pub struct GenerationalArrayEntry<T>
{
    // Kyren would set this to Option<T> but if we do so we can't get mutable references
    // to the internal data, only clones
    item : T, 
    generation : u32
}

/// This version also implements the storage for the thing being identified. 
/// 
/// It has some drawbacks: 
/// 
///  * You might have to resize an array with too many elements with possibly large storage
///  * You have to construct objects in the stack and then copy the entire content into the internal array
///  * You might end up with a lot of unused unrecoverable space after a lot of allocations
pub struct GenerationalIndexArray<T>
    where T : LifeCycle
{
    elements : Vec<GenerationalArrayEntry<T>>,
    free: Vec<usize>
}

impl<T> GenerationalIndexArray<T>
    where T : LifeCycle
{
    #[inline(always)]
    pub fn new() -> Self
    {
        GenerationalIndexArray { elements: vec![], free: vec![] }
    }

    pub fn store(&mut self, element : T) -> GenerationalIndex
    {
        if self.free.is_empty()
        {
            let next_index = self.elements.len();
            let entry = GenerationalArrayEntry{generation: 0, item: element};
            self.elements.push(entry);

            return GenerationalIndex{index: next_index, generation: 0};
        }

        let index = self.free.pop().unwrap();
        let entry = &mut self.elements[index];
        entry.item = element;

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
        self.elements[index.index].item.destroy();
    }

    pub fn get(&self, index: &GenerationalIndex) -> Option<&T>
    {
        if !self.is_live(index)
        {
            return None;
        }

        return Some(&self.elements[index.get_index()].item);
    }

    pub fn get_mut(&mut self, index: &GenerationalIndex) -> Option<&mut T>
    {
        if !self.is_live(index)
        {
            return None;
        }

        return Some(&mut self.elements[index.get_index()].item);
    }
}

// Te previous implementation has some problems about references and pointers. So instead 
// we will store pointers instead of the entire thing we are allocating.

