use crate::prefixed_storage::namespace_helpers::{
    get_with_prefix, range_with_prefix, remove_with_prefix, set_with_prefix,
};
use cosmwasm_std::{Order, Record, Storage};
use std::marker::PhantomData;

pub trait StoragePrefix {
    const PREFIX: &'static [u8];
}

pub struct TypedPrefixedStorage<'a, T> {
    storage: &'a dyn Storage,
    prefix: &'static [u8],
    data: PhantomData<T>,
}

impl<'a, T: StoragePrefix> TypedPrefixedStorage<'a, T> {
    pub fn new(storage: &'a dyn Storage) -> Self {
        Self {
            storage,
            prefix: T::PREFIX,
            data: PhantomData,
        }
    }
}

impl<T> Storage for TypedPrefixedStorage<'_, T> {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        get_with_prefix(self.storage, self.prefix, key)
    }

    fn range<'b>(
        &'b self,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'b> {
        range_with_prefix(self.storage, self.prefix, start, end, order)
    }

    fn set(&mut self, _key: &[u8], _value: &[u8]) {
        unimplemented!();
    }

    fn remove(&mut self, _key: &[u8]) {
        unimplemented!();
    }
}

pub struct TypedPrefixedStorageMut<'a, T> {
    storage: &'a mut dyn Storage,
    prefix: &'static [u8],
    data: PhantomData<T>,
}

impl<'a, T: StoragePrefix> TypedPrefixedStorageMut<'a, T> {
    pub fn new(storage: &'a mut dyn Storage) -> Self {
        Self {
            storage,
            prefix: T::PREFIX,
            data: PhantomData,
        }
    }
}

impl<T> Storage for TypedPrefixedStorageMut<'_, T> {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        get_with_prefix(self.storage, self.prefix, key)
    }

    fn range<'b>(
        &'b self,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'b> {
        range_with_prefix(self.storage, self.prefix, start, end, order)
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        set_with_prefix(self.storage, self.prefix, key, value);
    }

    fn remove(&mut self, key: &[u8]) {
        remove_with_prefix(self.storage, self.prefix, key);
    }
}
