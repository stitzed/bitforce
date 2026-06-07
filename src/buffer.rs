use std::ops::Deref;
use std::mem::MaybeUninit;

pub const MOVE_HISTORY_BUFFER_LEN: usize = 512;

/// A stack-based buffer with fixed capacity for primitives.
/// 
/// Uses uninitialized memory to avoid the cost of filling the buffer with default values.
/// Only the first `len` elements are guaranteed to be initialized.
/// # Examples
/// ```
/// # use bitforce::buffer::Buffer;
/// let mut buffer: Buffer<u8, 8> = Buffer::new();
/// 
/// buffer.push(1);
/// buffer.push(2);
/// 
/// assert_eq!(buffer.len(), 2);
/// 
/// assert_eq!(buffer.pop(), Some(2));
/// assert_eq!(buffer.len(), 1);
/// 
/// buffer.clear();
/// 
/// assert_eq!(buffer.is_empty(), true);
/// ```
pub struct Buffer<T, const N: usize> {
    buffer: [MaybeUninit<T>; N],
    len: usize
}

impl<T: Copy, const N: usize> Buffer<T, N> {
    /// Creates a new empty buffer on the stack with a fixed capacity.
    /// # Complexity
    /// `O(1)`
    #[inline(always)]
    pub fn new() -> Self {
        Self { buffer: [MaybeUninit::uninit(); N], len: 0 }
    }
    
    /// Appends an element to the back of the buffer.
    /// 
    /// # Panics
    /// Panics when trying to add an element with full capacity.
    /// # Complexity
    /// `O(1)`
    pub fn push(&mut self, item: T) {
        if self.len >= N {
            panic!("buffer overflow: capacity {} reached", N)
        }

        self.buffer[self.len].write(item);
        self.len += 1;
    }

    /// Returns the last element, if there is one, and removes it from the buffer.
    /// # Complexity
    /// `O(1)`
    pub fn pop(&mut self) -> Option<T> {
        if self.len > 0 {
            self.len -= 1;
            Some(unsafe {
                self.buffer[self.len].assume_init()
            })
        } else {
            None
        }
    }
    
    /// Clears the buffer.
    /// # Complexity
    /// `O(1)`
    #[inline(always)]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Returns the buffer as a slice.
    /// # Complexity
    /// `O(1)`
    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        unsafe {
            let arr_ptr: *const T = self.buffer.as_ptr() as *const T;
            std::slice::from_raw_parts(arr_ptr, self.len)
        }
    }
    
    /// Checks if the buffer is empty.
    /// # Complexity
    /// `O(1)`
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the length of the buffer.
    /// # Complexity
    /// `O(1)`
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }
}

impl<T: Copy, const N: usize> Deref for Buffer<T, N> {
    type Target = [T];
    
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}