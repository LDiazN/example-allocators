
#[cfg(test)]
mod tests
{
    mod kyren_tests
    {
        use crate::kyren_generational_indices::{self as kyren, GenerationalIndex, LifeCycle, GenerationalIndexArray};
    
        #[test]
        fn test_kyren_get()
        {
            let mut generational_indices = kyren::GenerationalIndices::new();
    
            let new_index = generational_indices.get();
            assert_eq!(new_index.get_index(), 0, "First index should start at 0"); 
            assert_eq!(new_index.get_generation(), 0, "First index should start at 0"); 
        }
    
        #[test]
        fn test_kyren_successive_gets()
        {
            let mut generational_indices = kyren::GenerationalIndices::new();
    
            let index_1 = generational_indices.get();
            let index_2 = generational_indices.get();
    
            assert!(index_1.get_index() != index_2.get_index());
    
            generational_indices.free(&index_1); // Index 1 should not be available now
            let index_1_new = generational_indices.get();
    
            assert_eq!(index_1_new.get_index(), index_1.get_index());
            assert_ne!(index_1_new.get_generation(), index_1.get_generation());
        }
    
        #[test]
        fn test_kyren_is_alive()
        {
            let mut generational_indices = kyren::GenerationalIndices::new();
    
            let index = generational_indices.get();
            assert!(generational_indices.is_live(&index));
            generational_indices.free(&index);
            assert!( !generational_indices.is_live(&index) );
        }
    
        /// Dummy implementation of an entity for testing purposes
        struct Entity
        {
            id : GenerationalIndex,
            name : String,
            is_active : bool
        }
    
        impl LifeCycle for Entity
        {
            fn destroy(&mut self) {
                self.is_active = false;
            }
    
            fn set_index(&mut self, index: GenerationalIndex) {
                self.id = index;
            }
        }
    
        #[test]
        fn test_kyren_get_array()
        {
            let mut generational_array = GenerationalIndexArray::<Entity>::new();
            
            let entity = Entity{
                name: "Entity1".to_string(),
                is_active: true,
                id: GenerationalIndex::default()
            };
    
            let index = generational_array.store(entity);
    
            let entity_ref = generational_array.get(&index);
            assert!(entity_ref.is_some());
            let entity_ref = generational_array.get_mut(&index);
            assert!(entity_ref.is_some());
        }
    
        #[test]
        fn test_kyren_free_array()
        {
            let mut generational_array = GenerationalIndexArray::<Entity>::new();
            let entity1 = Entity{
                name: "Entity1".to_string(),
                is_active: true,
                id: GenerationalIndex::default()
            };
    
            let index = generational_array.store(entity1);
            let entity_ref = generational_array.get(&index);
            assert!(entity_ref.is_some());
    
            generational_array.free(&index);
            let entity_ref = generational_array.get(&index);
            assert!(entity_ref.is_none());
        }
    
        #[test]
        fn test_kyren_replacement_array()
        {
            let mut generational_array = GenerationalIndexArray::<Entity>::new();
            let entity1 = Entity{
                name: "Entity1".to_string(),
                is_active: true,
                id: GenerationalIndex::default()
            };
    
            let entity2 = Entity{
                name: "Entity2".to_string(),
                is_active: true,
                id: GenerationalIndex::default()
            };
    
            let index = generational_array.store(entity1);
            generational_array.free(&index);
    
            let index = generational_array.store(entity2);
            let entity_ref = generational_array.get(&index);
    
            assert!(entity_ref.is_some());
            let entity_ref = entity_ref.unwrap();
            assert_eq!(entity_ref.name, "Entity2".to_string());
        }
    }
    
    // Memory allocators:
    mod memallocs_test
    {
        use crate::memory_allocators::*;

        struct Entity
        {
            id: usize, 
            is_active: bool,
            name: String
        }

        impl LifeCycle for Entity
        {
            fn new() -> Self {
                Self {
                    id: 0, 
                    is_active: false, 
                    name: "".to_string()
                }
            }

            fn reset(&mut self) {
                self.id = 0;
                self.is_active = false;
                self.name = "".to_string();
            }
        }

        #[test]
        fn test_mem_alloc_get_ptr()
        {
            let mut allocator = EntityAllocator::<Entity>::new();
    
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

            // Check that the entity will reset after free
            fn check_deallocated(entity : &mut Entity)
            {
                assert_eq!(entity.id, 0);
                assert_eq!(entity.name.as_str(), "");
                assert_eq!(entity.is_active, false);
            }
            allocator.allocate(check_deallocated);
        }
    }
}