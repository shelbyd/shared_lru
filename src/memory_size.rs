use std::mem::size_of;

pub trait MemorySize {
    fn bytes(&self) -> usize;
}

// TODO(shelbyd): Specialize for T: JustStack.
impl<T> MemorySize for Vec<T>
where
    T: MemorySize,
{
    fn bytes(&self) -> usize {
        size_of::<Self>()
            + self.iter().map(|v| v.bytes()).sum::<usize>()
            + (self.capacity() - self.len()) * size_of::<T>()
    }
}

impl<T: MemorySize> MemorySize for std::collections::VecDeque<T> {
    fn bytes(&self) -> usize {
        size_of::<Self>()
            + self.iter().map(|v| v.bytes()).sum::<usize>()
            + (self.capacity() - self.len()) * size_of::<T>()
    }
}

impl<K: MemorySize, V: MemorySize> MemorySize for std::collections::HashMap<K, V> {
    fn bytes(&self) -> usize {
        size_of::<Self>()
            + self
                .iter()
                .map(|(k, v)| k.bytes() + v.bytes())
                .sum::<usize>()
            + (self.capacity() - self.len()) * size_of::<(K, V)>()
    }
}

impl MemorySize for str {
    fn bytes(&self) -> usize {
        self.len()
    }
}

impl MemorySize for String {
    fn bytes(&self) -> usize {
        size_of::<Self>() + self.len()
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

impl<T> MemorySize for Option<T> where T: MemorySize {
    fn bytes(&self) -> usize {
        size_of::<Self>() + match self {
            Some(v) => MemorySize::bytes(v),
            None => 0,
        }
    }
}

#[cfg(feature = "serde_json")]
impl MemorySize for serde_json::Value {
    fn bytes(&self) -> usize {
        use serde_json::Value::*;

        let bonus = match self {
            Null | Bool(_) | Number(_) => 0,
            String(s) => s.len(),
            Array(arr) => arr.iter().map(MemorySize::bytes).sum(),
            Object(map) => map
                .iter()
                .map(|(key, value)| MemorySize::bytes(key) + MemorySize::bytes(value))
                .sum(),
        };

        size_of::<Self>() + bonus
    }
}

pub trait JustStack {}

impl JustStack for bool {}

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
impl<T: ?Sized> JustStack for &mut T {}

impl<A: JustStack> JustStack for (A,) {}
impl<A: JustStack, B: JustStack> JustStack for (A,B) {}
impl<A: JustStack, B: JustStack, C: JustStack> JustStack for (A,B,C) {}
