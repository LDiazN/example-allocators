// mod tests;
// mod kyren_generational_indices;
mod kyren_generational_indices;
mod memory_allocators;

struct Entity
{
    id: u64,
    name: &'static str
}

fn main() {
    let mut generational_pointers  = memory_allocators::GenerationalPointersArray::<Entity>::new();

    let entry = generational_pointers.allocate();
}
    
