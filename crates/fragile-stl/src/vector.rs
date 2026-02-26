pub trait std_vector_push_arg<T> {
    fn into_std_vector_value(self) -> T;
}

impl<T> std_vector_push_arg<T> for T {
    fn into_std_vector_value(self) -> T { self }
}

impl<T: Clone> std_vector_push_arg<T> for &T {
    fn into_std_vector_value(self) -> T { self.clone() }
}

// Generic std::vector<T> stub implementation backed by Vec<T>.
#[repr(C)]
#[derive(Default)]
pub struct std_vector<T> {
    inner: Vec<T>,
}

impl<T: Clone> Clone for std_vector<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> std_vector<T> {
    pub fn new_0() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn push_back<V>(&mut self, val: V)
    where
        V: std_vector_push_arg<T>,
    {
        self.inner.push(val.into_std_vector_value());
    }

    pub fn size(&self) -> usize { self.inner.len() }

    pub fn capacity(&self) -> usize { self.inner.capacity() }

    pub fn reserve(&mut self, new_cap: i32) {
        if new_cap <= 0 {
            return;
        }
        let target = new_cap as usize;
        if target > self.inner.capacity() {
            self.inner.reserve(target - self.inner.capacity());
        }
    }

    pub fn resize(&mut self, new_size: i32)
    where
        T: Default,
    {
        if new_size <= 0 {
            self.inner.clear();
            return;
        }
        self.inner.resize_with(new_size as usize, T::default);
    }

    pub fn back(&mut self) -> &mut T {
        self.inner
            .last_mut()
            .expect("std_vector::back called on empty vector")
    }

    pub fn front(&mut self) -> &mut T {
        self.inner
            .first_mut()
            .expect("std_vector::front called on empty vector")
    }

    pub fn begin(&mut self) -> *mut T {
        self.inner.as_mut_ptr()
    }

    pub fn end(&mut self) -> *mut T {
        unsafe { self.inner.as_mut_ptr().add(self.inner.len()) }
    }

    pub fn data(&self) -> *const T {
        self.inner.as_ptr()
    }

    pub fn empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

impl<T> IntoIterator for std_vector<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

pub type std_vector_int = std_vector<i32>;
