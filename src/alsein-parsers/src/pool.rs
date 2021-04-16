use std::{collections::HashMap, marker::PhantomData, mem, sync::Mutex};

#[derive(Default)]
pub struct Pool<'a> {
    values: Mutex<HashMap<(usize, usize), unsafe fn((usize, usize))>>,
    _phantom: PhantomData<&'a ()>,
}

unsafe fn drop_ptr<T: ?Sized>(ptr: (usize, usize)) {
    let ptr = &mut **mem::transmute::<_, *mut *mut T>(&ptr);
    Box::from_raw(ptr);
}

impl<'a> Pool<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    fn stored_ptr<T: ?Sized>(ptr: *mut T) -> (usize, usize) {
        unsafe {
            let ptr_extra = (ptr, 0usize);
            **mem::transmute::<_, *mut *mut (usize, usize)>(&ptr_extra)
        }
    }

    pub fn add<T: ?Sized>(&self, item: Box<T>) -> &'a mut T {
        unsafe {
            let ptr = Box::into_raw(item);
            let mut values = self.values.lock().unwrap();
            values.insert(Self::stored_ptr(ptr), drop_ptr::<T>);
            &mut *ptr
        }
    }

    pub fn remove<T: ?Sized>(&'a self, item: &mut T) -> Option<Box<T>> {
        unsafe {
            let ptr = Self::stored_ptr(item);
            let mut values = self.values.lock().unwrap();
            values.remove(&ptr).map(move |_| Box::from_raw(item))
        }
    }
}

impl<'a> Drop for Pool<'a> {
    fn drop(&mut self) {
        let values = self.values.lock().unwrap();
        for (ptr, drop_ptr) in &*values {
            unsafe {
                drop_ptr(*ptr);
            }
        }
    }
}
