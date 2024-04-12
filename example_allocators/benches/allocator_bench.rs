
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use example_allocators::{*, memory_allocators::{EntityPtr, EntityAllocator, GIABoxUninit}};

#[derive(Default)]
struct Entity
{
    id : usize, 
    is_active : bool,
    name : String
}

fn generational_array_allocation_benchmark(c: &mut Criterion) 
{
    const N_ENTITIES : u64 = 100;
    c.bench_function("Generational Pointers: Entity Allocation 100", |b| b.iter(|| {
        let mut gpa = memory_allocators::GIABoxUninit::<Entity>::default();
        for i in 0..N_ENTITIES
        {
            let entity = gpa.new(Entity::default());
            let mut entity_ref = gpa.get(&entity).unwrap().borrow_mut();
            entity_ref.id = i as usize;
            entity_ref.name = "Testing".to_string();
            entity_ref.is_active = true;
        }
    }));
}

fn entity_allocator_allocation_benchmark(c: &mut Criterion)
{
    const N_ENTITIES : u64 = 10_000;
    c.bench_function("Memory Allocator: Entity Allocation 10k", |b| b.iter(|| {
        let mut en_alloc = memory_allocators::EntityAllocator::<Entity>::default();
        for i in 0..N_ENTITIES
        {
            en_alloc.allocate(|entity| {
                entity.name = "Testing".to_string();
                entity.id = i as usize;
                entity.is_active = true;
            });
        }
    }));
}

fn entity_allocator_access_benchmark(c: &mut Criterion)
{
    const N_ENTITIES : u64 = 100;
    

    fn benched_fn((pointers, _alloc) : (Vec<EntityPtr<Entity>>,  EntityAllocator<Entity>))
    {
        for ptr in pointers
        {
            let _id = black_box(ptr.id);
            let _name = black_box(&ptr.name);
            let _is_active = black_box(ptr.is_active);
        }
    }

    c.bench_function("Memory Allocator: Entity Access 100", |b| b.iter(move || benched_fn({
        let mut en_alloc = memory_allocators::EntityAllocator::<Entity>::default();
        let mut pointers = Vec::with_capacity(N_ENTITIES as usize);
        for i in 0..N_ENTITIES
        {
            pointers.push(en_alloc.allocate(|entity| {
                entity.name = "Testing".to_string();
                entity.id = i as usize;
                entity.is_active = true;
            }));
        }
        (pointers, en_alloc)
    })));
}

fn pointers_array_access_benchmark(c: &mut Criterion)
{
    const N_ENTITIES : u64 = 100;
    
    fn benched_fn((pointers, _alloc) : (Vec<memory_allocators::GenerationalIndex>, GIABoxUninit<Entity>))
    {
        let mut _alloc = _alloc;
        for ptr in pointers
        {
            let entity_ref = _alloc.get(&ptr).unwrap().borrow_mut();
            let _id = black_box(entity_ref.id);
            let _name = black_box(&entity_ref.name);
            let _is_active = black_box(entity_ref.is_active);
        }
    }

    c.bench_function("Pointers Array: Entity Access 100", |b| b.iter(move || benched_fn({
        let mut alloc = memory_allocators::GIABoxUninit::<Entity>::default();
        let mut pointers = Vec::with_capacity(N_ENTITIES as usize);

        for i in 0..N_ENTITIES
        {
            let handle = alloc.new(Entity::default());
            let mut entity_ref = alloc.get(&handle).unwrap().borrow_mut();
            entity_ref.name = "Testing".to_string();
            entity_ref.id = i as usize;
            entity_ref.is_active = true;

            pointers.push(handle);
        }
        (pointers, alloc)
    })));
}

fn inplace_mem_alloc_allocation_benchmark(c : &mut Criterion)
{
    const N_ENTITIES : u64 = 10_000;
    c.bench_function("inplace allocator: Allocation 10k", |b| b.iter(|| {
        let mut alloc = memory_allocators::InPlaceMemoryAllocator::<Entity>::default();
        for _ in 0..N_ENTITIES
        {
            let entity_handle = alloc.allocate();
            let entity_ref = alloc.get(&entity_handle);
            entity_ref.id = 42;
            entity_ref.name = "test".to_owned();
            entity_ref.is_active = true;
        }
    }));
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(50);
    targets = 
                generational_array_allocation_benchmark, 
                entity_allocator_allocation_benchmark, 
                entity_allocator_access_benchmark, 
                pointers_array_access_benchmark,
                inplace_mem_alloc_allocation_benchmark
);
criterion_main!(benches);