#![feature(generic_const_exprs)]
use wasmi::{Memory, Store, AsContext, AsContextMut};

// TODO use Results instead of Options
pub fn read_bytes(memory: &Memory, store: &impl AsContext, offset: usize, length: usize) -> Option<Vec<u8>> {
    let mut bytes = vec![0u8; length];
    memory.read(store, offset, bytes.as_mut_slice()).ok()?;

    Some(bytes)
}

pub fn read_string(memory: &Memory, store: &impl AsContext, offset: usize, length: usize) -> Option<String> {
    let bytes = read_bytes(memory, store, offset, length)?;
    String::from_utf8(bytes).ok()
}

pub fn read_values<T: FromLeBytes + Sized>(memory: &Memory, store: &impl AsContext, offset: usize, count: usize) -> Option<Vec<T>> {
    let element_size = std::mem::size_of::<T>();
    let bytes_size = element_size * count;
    let mut buf: Vec<u8> = vec![0u8; bytes_size];
    memory.read(store, offset, buf.as_mut_slice()).ok()?;

    let elements: Vec<T> = buf.chunks(element_size)
        .map(|element_bytes| T::from_le_bytes(element_bytes))
        .collect();

    Some(elements)
}

pub fn write_bytes(memory: &Memory, store: &mut impl AsContextMut, buffer: &[u8], offset: usize) -> Option<()> {
    memory.write(store, offset, buffer).ok()
}

pub trait FromLeBytes : Sized {
    fn from_le_bytes(bytes: &[u8]) -> Self;
}

macro_rules! from_le_bytes {
    ($t:ty) => {
        impl FromLeBytes for $t {
            fn from_le_bytes(bytes: &[u8]) -> Self {
                assert_eq!(std::mem::size_of::<$t>(), bytes.len());

                <$t>::from_le_bytes(bytes.try_into().unwrap())
            }
        }
    };
}

from_le_bytes!(usize);
from_le_bytes!(u8);
from_le_bytes!(u16);
from_le_bytes!(u32);
from_le_bytes!(u64);
from_le_bytes!(i8);
from_le_bytes!(i16);
from_le_bytes!(i32);
from_le_bytes!(i64);
