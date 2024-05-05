use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use example_allocators::{
    memory_allocators::{GIABoxUninit, InPlaceAllocator},
    *,
};

#[derive(Default)]
struct Entity {
    id: usize,
    is_active: bool,
    name: String,
}

fn generational_array_allocation_bench(c: &mut Criterion) {
    const N_ENTITIES: u64 = 100;
    c.bench_function("Generational Pointers: Entity Allocation 100", |b| {
        b.iter(|| {
            let mut gpa = memory_allocators::GIABoxUninit::<Entity>::default();
            for i in 0..N_ENTITIES {
                let entity = gpa.new(Entity::default());
                let mut entity_ref = gpa.get(&entity).unwrap().borrow_mut();
                entity_ref.id = i as usize;
                entity_ref.name = "Testing".to_string();
                entity_ref.is_active = true;
            }
        })
    });
}

fn pointers_array_access_bench(c: &mut Criterion) {
    const N_ENTITIES: u64 = 100;

    fn benched_fn(
        (pointers, _alloc): (
            Vec<memory_allocators::GenerationalIndex>,
            GIABoxUninit<Entity>,
        ),
    ) {
        let mut _alloc = _alloc;
        for ptr in pointers {
            let entity_ref = _alloc.get(&ptr).unwrap().borrow_mut();
            let _id = black_box(entity_ref.id);
            let _name = black_box(&entity_ref.name);
            let _is_active = black_box(entity_ref.is_active);
        }
    }

    c.bench_function("Pointers Array: Entity Access 100", |b| {
        b.iter(move || {
            benched_fn({
                let mut alloc = memory_allocators::GIABoxUninit::<Entity>::default();
                let mut pointers = Vec::with_capacity(N_ENTITIES as usize);

                for i in 0..N_ENTITIES {
                    let handle = alloc.new(Entity::default());
                    let mut entity_ref = alloc.get(&handle).unwrap().borrow_mut();
                    entity_ref.name = "Testing".to_string();
                    entity_ref.id = i as usize;
                    entity_ref.is_active = true;

                    pointers.push(handle);
                }
                (pointers, alloc)
            })
        })
    });
}

fn box_alloc_allocation_bench(c: &mut Criterion) {
    const N_ENTITIES: u64 = 10_000;
    c.bench_function("Box Allocator: Entity Allocation 10k", |b| {
        b.iter(|| {
            let mut box_alloc = memory_allocators::BoxAllocator::<Entity>::default();
            for i in 0..N_ENTITIES {
                box_alloc.new(Entity {
                    name: "Testing".to_string(),
                    id: i as usize,
                    is_active: true,
                });
            }
        })
    });
}

fn box_alloc_access_bench(c: &mut Criterion) {
    const N_ENTITIES: u64 = 10_000;

    let mut en_alloc = memory_allocators::BoxAllocator::<Entity>::default();
    let mut pointers = Vec::with_capacity(N_ENTITIES as usize);
    for i in 0..N_ENTITIES {
        pointers.push(en_alloc.new(Entity {
            name: "Testing".to_string(),
            id: i as usize,
            is_active: true,
        }));
    }
    let input = (pointers, en_alloc);

    c.bench_with_input(BenchmarkId::new("Box Allocator: Access 10k", "10k Pointers parameter"),&input, |b, input| {
        let (pointers, _alloc) = input;
        b.iter(move||{
            for ptr in pointers {
                let _id = black_box(ptr.id);
                let _name = black_box(&ptr.name);
                let _is_active = black_box(ptr.is_active);
            }
        })
    });
}

fn inplace_alloc_allocation_bench(c: &mut Criterion) {
    const N_ENTITIES: u64 = 10_000;
    c.bench_function("InPlace allocator: Allocation 10k", |b| {
        b.iter(|| {
            let mut alloc = memory_allocators::InPlaceAllocator::<Entity>::default();
            for _ in 0..N_ENTITIES {
                let entity_handle = alloc.new(Entity::default());
                let entity_ref = alloc.get(&entity_handle);
                entity_ref.id = 42;
                entity_ref.name = "test".to_owned();
                entity_ref.is_active = true;
            }
        })
    });
}

fn inplace_alloc_access_bench(c: &mut Criterion) {
    const N_ENTITIES: u64 = 10_000;
    let mut alloc = InPlaceAllocator::<Entity>::default();
    let mut handles = vec![];

    for _ in 0..N_ENTITIES {
        let new_handle = alloc.new(Entity::default());
        handles.push(new_handle);
    }

    let input = (alloc, handles);

    c.bench_with_input(BenchmarkId::new("InPlace allocator: Access 10k", "10k Allocated entities"), {
        &input
       },
       |b, input| {
        b.iter(move || {
            let (alloc, handles) = &input;
            for handle in handles.iter() {
                let _id = black_box(alloc.get(handle));
                let _name = black_box(alloc.get(handle));
                let _is_active = black_box(alloc.get(handle));
            }
        })
       }
    );
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(50);
    targets =   generational_array_allocation_bench,
                pointers_array_access_bench,

                box_alloc_allocation_bench,
                box_alloc_access_bench,
                inplace_alloc_allocation_bench,
                inplace_alloc_access_bench
);
criterion_main!(benches);
