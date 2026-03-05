pub trait std_deque_push_arg<T> {
    fn into_std_deque_value(self) -> T;
}

impl<T> std_deque_push_arg<T> for T {
    fn into_std_deque_value(self) -> T {
        self
    }
}

impl<T: Clone> std_deque_push_arg<T> for &T {
    fn into_std_deque_value(self) -> T {
        self.clone()
    }
}

// Generic std::deque<T> stub implementation backed by VecDeque<T>.
#[repr(C)]
#[derive(Default)]
pub struct std_deque<T> {
    inner: std::collections::VecDeque<T>,
}

impl<T: Clone> Clone for std_deque<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> std_deque<T> {
    pub fn new_0() -> Self {
        Self {
            inner: std::collections::VecDeque::new(),
        }
    }

    pub fn empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn size(&self) -> usize {
        self.inner.len()
    }

    pub fn push_back<V>(&mut self, val: V)
    where
        V: std_deque_push_arg<T>,
    {
        self.inner.push_back(val.into_std_deque_value());
    }

    pub fn push_front<V>(&mut self, val: V)
    where
        V: std_deque_push_arg<T>,
    {
        self.inner.push_front(val.into_std_deque_value());
    }

    pub fn pop_back(&mut self) {
        let _ = self.inner.pop_back();
    }

    pub fn pop_front(&mut self) {
        let _ = self.inner.pop_front();
    }

    pub fn front(&mut self) -> &mut T {
        self.inner
            .front_mut()
            .expect("std_deque::front called on empty deque")
    }

    pub fn back(&mut self) -> &mut T {
        self.inner
            .back_mut()
            .expect("std_deque::back called on empty deque")
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

pub trait std_queue_push_arg<T> {
    fn into_std_queue_value(self) -> T;
}

impl<T> std_queue_push_arg<T> for T {
    fn into_std_queue_value(self) -> T {
        self
    }
}

impl<T: Clone> std_queue_push_arg<T> for &T {
    fn into_std_queue_value(self) -> T {
        self.clone()
    }
}

// Generic std::queue<T> stub implementation backed by VecDeque<T>.
#[repr(C)]
#[derive(Default)]
pub struct std_queue<T> {
    inner: std::collections::VecDeque<T>,
}

impl<T: Clone> Clone for std_queue<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> std_queue<T> {
    pub fn new_0() -> Self {
        Self {
            inner: std::collections::VecDeque::new(),
        }
    }

    pub fn empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn size(&self) -> usize {
        self.inner.len()
    }

    pub fn push<V>(&mut self, val: V)
    where
        V: std_queue_push_arg<T>,
    {
        self.inner.push_back(val.into_std_queue_value());
    }

    pub fn push_back<V>(&mut self, val: V)
    where
        V: std_queue_push_arg<T>,
    {
        self.push(val);
    }

    pub fn pop(&mut self) {
        let _ = self.inner.pop_front();
    }

    pub fn front(&mut self) -> &mut T {
        self.inner
            .front_mut()
            .expect("std_queue::front called on empty queue")
    }

    pub fn back(&mut self) -> &mut T {
        self.inner
            .back_mut()
            .expect("std_queue::back called on empty queue")
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

pub trait std_stack_push_arg<T> {
    fn into_std_stack_value(self) -> T;
}

impl<T> std_stack_push_arg<T> for T {
    fn into_std_stack_value(self) -> T {
        self
    }
}

impl<T: Clone> std_stack_push_arg<T> for &T {
    fn into_std_stack_value(self) -> T {
        self.clone()
    }
}

// Generic std::stack<T> stub implementation backed by std::vec::Vec<T>.
#[repr(C)]
#[derive(Default)]
pub struct std_stack<T> {
    inner: std::vec::Vec<T>,
}

impl<T: Clone> Clone for std_stack<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> std_stack<T> {
    pub fn new_0() -> Self {
        Self {
            inner: std::vec::Vec::new(),
        }
    }

    pub fn empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn size(&self) -> usize {
        self.inner.len()
    }

    pub fn push<V>(&mut self, val: V)
    where
        V: std_stack_push_arg<T>,
    {
        self.inner.push(val.into_std_stack_value());
    }

    pub fn push_back<V>(&mut self, val: V)
    where
        V: std_stack_push_arg<T>,
    {
        self.push(val);
    }

    pub fn pop(&mut self) {
        let _ = self.inner.pop();
    }

    pub fn top(&mut self) -> &mut T {
        self.inner
            .last_mut()
            .expect("std_stack::top called on empty stack")
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

pub type std_queue_int = std_queue<i32>;
pub type std_stack_int = std_stack<i32>;
pub type std_deque_int = std_deque<i32>;
