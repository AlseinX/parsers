use std::{collections::HashMap, marker::PhantomData, mem};

pub struct Pool<'a> {
    values: HashMap<(usize, usize), unsafe fn((usize, usize))>,
    _phantom: PhantomData<&'a u8>,
}

unsafe fn drop_ptr<T: ?Sized>(ptr: (usize, usize)) {
    let ptr = &mut **mem::transmute::<_, *mut *mut T>(&ptr);
    Box::from_raw(ptr);
}

impl<'a> Pool<'a> {
    fn stored_ptr<T: ?Sized + 'a>(ptr: *mut T) -> (usize, usize) {
        unsafe {
            let ptr_extra = (ptr, 0usize);
            **mem::transmute::<_, *mut *mut (usize, usize)>(&ptr_extra)
        }
    }

    pub fn add<T: ?Sized + 'a>(&mut self, item: Box<T>) -> &mut T {
        unsafe {
            let ptr = Box::into_raw(item);
            self.values.insert(Self::stored_ptr(ptr), drop_ptr::<T>);
            &mut *ptr
        }
    }

    pub fn remove<T: ?Sized + 'a>(&mut self, item: &mut T) -> Option<Box<T>> {
        unsafe {
            let ptr = Self::stored_ptr(item);
            self.values.remove(&ptr).map(move |_| Box::from_raw(item))
        }
    }
}

impl<'a> Drop for Pool<'a> {
    fn drop(&mut self) {
        for (ptr, drop_ptr) in &self.values {
            unsafe {
                drop_ptr(*ptr);
            }
        }
    }
}
