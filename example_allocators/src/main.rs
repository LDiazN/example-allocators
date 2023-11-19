use std::mem::MaybeUninit;

mod tests;
mod kyren_generational_indices;
mod memory_allocators; 

struct Entity
{
    id: usize,
    name: String,
    active: bool
}

fn main() {
    let mu = MaybeUninit::<Entity>::uninit();
    assert_eq!(std::mem::size_of_val(&mu), std::mem::size_of::<Entity>());
}
    
