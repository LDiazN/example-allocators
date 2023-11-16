// mod tests;
// mod kyren_generational_indices;
use std::alloc::{alloc, Layout};

struct Thing{
    a: usize, 
    id: u32
}

impl Thing
{
    fn from_mem(mem : *mut Thing)
    {
        unsafe
        {
            let thing_ref = mem.as_mut().unwrap();
            thing_ref.a = 69;
            thing_ref.id = 42;
        }
    }

    fn new() -> Thing
    {
        Thing { a: 69, id: 42 }
    }
}
struct HeaderThing
{
    generation: u16,
    thing: Thing
}

fn main() {

    let x :&Thing = unsafe 
    {

        let thing_mem = alloc(Layout::new::<HeaderThing>());
        {
            thing_mem.cast::<HeaderThing>().as_mut().unwrap().generation = 420;
        }
        let header_size = std::mem::size_of::<u16>();
        let thing_align = std::mem::align_of::<Thing>();
        let offset = header_size + header_size  % thing_align;
        let thing_part_mem = thing_mem.add(offset).cast::<Thing>();
        
        Thing::from_mem(thing_part_mem);

        thing_part_mem.as_mut().unwrap()
    };


    assert_eq!(x.a, 69);
    assert_eq!(x.id, 42);
    unsafe 
    {
        let thing_ptr = x as *const Thing;
        let header_size = std::mem::size_of::<u16>();
        let thing_align = std::mem::align_of::<Thing>();
        let offset = header_size + header_size  % thing_align;
        let header_start = thing_ptr
                    .cast::<u8>()
                    .sub(offset)
                    .cast::<HeaderThing>();

        assert_eq!(header_start.as_ref().unwrap().generation, 420);
    }
}
    
