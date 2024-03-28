
#[cfg(test)]
mod tests
{
    mod kyren_tests
    {
        use crate::kyren_generational_indices::{self as kyren, GenerationalIndex, GenerationalIndexArray, GenerationalIndexArrayCell};

        #[test]
        fn test_kyren_get()
        {
            let mut generational_indices = kyren::GenerationalIndices::default();
    
            let new_index = generational_indices.new();
            assert_eq!(new_index.get_index(), 0, "First index should start at 0"); 
            assert_eq!(new_index.get_generation(), 0, "First index should start at 0"); 
        }
    
        #[test]
        fn test_kyren_successive_gets()
        {
            let mut generational_indices = kyren::GenerationalIndices::default();
    
            let index_1 = generational_indices.new();
            let index_2 = generational_indices.new();
    
            assert!(index_1.get_index() != index_2.get_index());
    
            generational_indices.free(&index_1); // Index 1 should not be available now
            let index_1_new = generational_indices.new();
    
            assert_eq!(index_1_new.get_index(), index_1.get_index());
            assert_ne!(index_1_new.get_generation(), index_1.get_generation());
        }
    
        #[test]
        fn test_kyren_is_alive()
        {
            let mut generational_indices = kyren::GenerationalIndices::default();
    
            let index = generational_indices.new();
            assert!(generational_indices.is_live(&index));
            generational_indices.free(&index);
            assert!( !generational_indices.is_live(&index) );
        }
    
        /// Dummy implementation of an entity for testing purposes
        #[derive(Debug, Default)]
        struct Entity
        {
            _id : GenerationalIndex,
            name : String,
            _is_active : bool
        }
    
        #[test]
        fn test_kyren_get_array()
        {
            let mut generational_array = GenerationalIndexArray::<Entity>::default();
            
            let entity = Entity{
                name: "Entity1".to_string(),
                _is_active: true,
                _id: GenerationalIndex::default()
            };
    
            let index = generational_array.new(entity);
    
            let entity_ref = generational_array.get(&index);
            assert!(entity_ref.is_some());
            let entity_ref = generational_array.get_mut(&index);
            assert!(entity_ref.is_some());
        }
    
        #[test]
        fn test_kyren_free_array()
        {
            let mut generational_array = GenerationalIndexArray::<Entity>::default();
            let entity1 = Entity{
                name: "Entity1".to_string(),
                _is_active: true,
                _id: GenerationalIndex::default()
            };
    
            let index = generational_array.new(entity1);
            let entity_ref = generational_array.get(&index);
            assert!(entity_ref.is_some());
    
            generational_array.free(&index);
            let entity_ref = generational_array.get(&index);
            assert!(entity_ref.is_none());
        }
    
        #[test]
        fn test_kyren_replacement_array()
        {
            let mut generational_array = GenerationalIndexArray::<Entity>::default();
            let entity1 = Entity{
                name: "Entity1".to_string(),
                _is_active: true,
                _id: GenerationalIndex::default()
            };
    
            let entity2 = Entity{
                name: "Entity2".to_string(),
                _is_active: true,
                _id: GenerationalIndex::default()
            };
    
            let index = generational_array.new(entity1);
            generational_array.free(&index);
    
            let index = generational_array.new(entity2);
            let entity_ref = generational_array.get(&index);
    
            assert!(entity_ref.is_some());
            let entity_ref = entity_ref.unwrap();
            assert_eq!(entity_ref.name, "Entity2".to_string());
        }

        // -- PTR based allocator
        #[test]
        fn test_kyren_ptr_array()
        {
            let mut allocator : GenerationalIndexArrayCell<Entity>= GenerationalIndexArrayCell::default();

            let genid1 = allocator.new(Entity{
                    name: "e1".to_string(),
                    _is_active: false,
                    _id: GenerationalIndex::default()}
                );
            let genid2 = allocator.new(Entity{
                    name: "e2".to_string(),
                    _is_active: false,
                    _id: GenerationalIndex::default()}
                );
            
            let entity1 = allocator.get(&genid1).unwrap();
            let entity2 = allocator.get(&genid2).unwrap();

            entity1.borrow_mut()._is_active = true;
            entity2.borrow_mut()._is_active = true;
        }

    }
    
    // Memory allocators:
    mod memallocs_test
    {
        use std::process::id;

        use crate::{kyren_generational_indices::GIAUninitCell, memory_allocators::*};

        struct Entity
        {
            id: usize, 
            is_active: bool,
            name: String
        }

        impl Default for Entity
        {
            fn default() -> Self {
                Self { id: 0, is_active: false, name: "".to_string() }
            }
        }

        #[test]
        fn test_generational_pointer_array_allocate()
        {
            let mut gpa = GenerationalPointersArray::<Entity>::default();
            let entity_handle = gpa.allocate();

            // Try initialize it 
            {
                let entity_ref = gpa.get(&entity_handle);
                assert!(entity_ref.is_some());
                let entity_ref = entity_ref.unwrap();

                entity_ref.id = 42;
                entity_ref.name = "test1".to_owned();
                entity_ref.is_active = true;
            }

            // Check that lookup works properly, we're looking up the same entity that was
            // initialized in the step before
            {
                let entity_ref = gpa.get(&entity_handle);
                assert!(entity_ref.is_some());
                let entity_ref = entity_ref.unwrap();
                assert_eq!(entity_ref.id, 42);
                assert_eq!(entity_ref.name.as_str(), "test1");
                assert_eq!(entity_ref.is_active, true);
            }
        }

        #[test]   
        fn test_generational_pointer_array_free()
        {
            let mut gpa = GenerationalPointersArray::<Entity>::default();
            let entity_handle = gpa.allocate();

            assert!(gpa.get(&entity_handle).is_some());
            gpa.free(&entity_handle);
            assert!(gpa.get(&entity_handle).is_none());
        }

        #[test]   
        #[should_panic]
        fn test_generational_pointer_array_double_free()
        {
            let mut gpa = GenerationalPointersArray::<Entity>::default();
            let entity_handle = gpa.allocate();

            gpa.free(&entity_handle);
            gpa.free(&entity_handle);
        }

        #[test]
        fn test_mem_alloc_get_ptr()
        {
            let mut allocator = EntityAllocator::<Entity>::default();
    
            fn init_fn(entity : &mut Entity)
            {
                entity.name = "Example1".to_string();
                entity.is_active = true;
                entity.id = 42;
            }
    
            let entity = allocator.allocate(init_fn);

            // Check that initialization works properly
            assert_eq!(entity.name.as_str(), "Example1");
            assert_eq!(entity.is_active, true);
            assert_eq!(entity.id, 42);

            // Check that the EntityPtr knows when it's dead
            assert!(entity.is_live());
            allocator.free(&entity);
            assert!(!entity.is_live());
        }

        #[test]
        fn test_inplace_mem_alloc_alloc()
        {
            let mut inplace_alloc = InPlaceMemoryAllocator::<Entity>::default();
            let entity_handle = inplace_alloc.allocate();
            {
                let entity_ref = inplace_alloc.get(&entity_handle);
                entity_ref.id = 42;
                entity_ref.is_active = true;
                entity_ref.name = "test".to_owned();
            }

            let entity_ref = inplace_alloc.get(&entity_handle);
            assert_eq!(entity_ref.id, 42);
            assert_eq!(entity_ref.is_active, true);
            assert_eq!(entity_ref.name.as_str(), "test");
        }

        #[test]
        fn test_inplace_mem_alloc_free()
        {
            let mut inplace_alloc = InPlaceMemoryAllocator::<Entity>::default();
            let entity_handle = inplace_alloc.allocate();
            assert!(inplace_alloc.is_live(&entity_handle));
            inplace_alloc.free(&entity_handle);
            assert!(!inplace_alloc.is_live(&entity_handle));
        }

        #[test]
        #[should_panic]
        fn test_inplace_mem_alloc_get_free()
        {
            let mut inplace_alloc = InPlaceMemoryAllocator::<Entity>::default();
            let entity_handle = inplace_alloc.allocate();

            assert!(inplace_alloc.is_live(&entity_handle));
            inplace_alloc.free(&entity_handle);
            inplace_alloc.get(&entity_handle); // boom
        }


        // Uninit Cell Generational array 
        #[test]
        fn test_uninit_cell_gia() 
        {
            let mut uninit_cell_gia = GIAUninitCell::<Entity>::default();
            let entity1 = Entity{name: "entity1".to_string(), id: 0, is_active: true};

            let idx1 = uninit_cell_gia.new(entity1);


            assert!(uninit_cell_gia.is_live(&idx1));
            
            // Accessing the entity
            unsafe {
                let entity1_ref = uninit_cell_gia.get(&idx1).unwrap();
                let mut uninit = entity1_ref.borrow_mut();
                let entity = uninit.assume_init_mut(); // unsafe
                entity.id = 42;
                entity.is_active = true;
            }

            // deleting entity
            uninit_cell_gia.free(&idx1);

            assert!(!uninit_cell_gia.is_live(&idx1));
        }
    }
}