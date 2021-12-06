use std::mem::size_of;

pub trait MemorySize {
    fn bytes(&self) -> usize;
}

impl<T> MemorySize for Vec<T>
where
    T: MemorySize,
{
    fn bytes(&self) -> usize {
        let stack = size_of::<Self>();
        let occupied = self.iter().map(|v| v.bytes()).sum::<usize>();
        let unoccupied = (self.capacity() - self.len()) * size_of::<T>();

        stack + occupied + unoccupied
    }
}

impl MemorySize for str {
    fn bytes(&self) -> usize {
        self.len()
    }
}

impl<T> MemorySize for T
where
    T: JustStack,
{
    fn bytes(&self) -> usize {
        size_of::<Self>()
    }
}

pub trait JustStack {}

impl JustStack for u8 {}
impl JustStack for u16 {}
impl JustStack for u32 {}
impl JustStack for u64 {}
impl JustStack for usize {}

impl JustStack for i8 {}
impl JustStack for i16 {}
impl JustStack for i32 {}
impl JustStack for i64 {}
impl JustStack for isize {}

impl<T: ?Sized> JustStack for &T {}

impl<T: JustStack> JustStack for Option<T> {}
impl<R: JustStack, E: JustStack> JustStack for Result<R, E> {}
