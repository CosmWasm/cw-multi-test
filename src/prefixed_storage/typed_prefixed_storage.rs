use crate::prefixed_storage::length_prefixed::{to_length_prefixed, to_length_prefixed_nested};
use crate::prefixed_storage::namespace_helpers::{
    get_with_prefix, range_with_prefix, remove_with_prefix, set_with_prefix,
};
use crate::prefixed_storage::{PrefixedStorage, ReadonlyPrefixedStorage};
use cosmwasm_std::{Order, Record, Storage};
use std::marker::PhantomData;

pub trait StoragePrefix {
    const NAMESPACE: &'static [u8];
}

pub struct TypedPrefixedStorage<'a, T: StoragePrefix> {
    storage: &'a dyn Storage,
    prefix: Vec<u8>,
    _data: PhantomData<T>,
}

impl<'a, T: StoragePrefix> TypedPrefixedStorage<'a, T> {
    pub fn new(storage: &'a dyn Storage) -> Self {
        Self {
            storage,
            prefix: to_length_prefixed(T::NAMESPACE),
            _data: PhantomData,
        }
    }

    pub fn multilevel(storage: &'a dyn Storage, namespace: &[u8]) -> Self {
        Self {
            storage,
            prefix: to_length_prefixed_nested(&[T::NAMESPACE, namespace]),
            _data: PhantomData,
        }
    }
}

impl<T: StoragePrefix> Storage for TypedPrefixedStorage<'_, T> {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        get_with_prefix(self.storage, &self.prefix, key)
    }

    fn range<'x>(
        &'x self,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'x> {
        range_with_prefix(self.storage, &self.prefix, start, end, order)
    }

    fn set(&mut self, _key: &[u8], _value: &[u8]) {
        unreachable!();
    }

    fn remove(&mut self, _key: &[u8]) {
        unreachable!();
    }
}

impl<'a, T: StoragePrefix> From<TypedPrefixedStorage<'a, T>> for ReadonlyPrefixedStorage<'a> {
    fn from(value: TypedPrefixedStorage<'a, T>) -> Self {
        Self {
            storage: value.storage,
            prefix: value.prefix,
        }
    }
}

pub struct TypedPrefixedStorageMut<'a, T: StoragePrefix> {
    storage: &'a mut dyn Storage,
    prefix: Vec<u8>,
    _data: PhantomData<T>,
}

impl<'a, T: StoragePrefix> TypedPrefixedStorageMut<'a, T> {
    pub fn new(storage: &'a mut dyn Storage) -> Self {
        Self {
            storage,
            prefix: to_length_prefixed(T::NAMESPACE),
            _data: PhantomData,
        }
    }

    pub fn multilevel(storage: &'a mut dyn Storage, namespace: &[u8]) -> Self {
        Self {
            storage,
            prefix: to_length_prefixed_nested(&[T::NAMESPACE, namespace]),
            _data: PhantomData,
        }
    }
}

impl<T: StoragePrefix> Storage for TypedPrefixedStorageMut<'_, T> {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        get_with_prefix(self.storage, &self.prefix, key)
    }

    fn range<'y>(
        &'y self,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'y> {
        range_with_prefix(self.storage, &self.prefix, start, end, order)
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        set_with_prefix(self.storage, &self.prefix, key, value);
    }

    fn remove(&mut self, key: &[u8]) {
        remove_with_prefix(self.storage, &self.prefix, key);
    }
}

impl<'a, T: StoragePrefix> From<TypedPrefixedStorageMut<'a, T>> for PrefixedStorage<'a> {
    fn from(value: TypedPrefixedStorageMut<'a, T>) -> Self {
        Self {
            storage: value.storage,
            prefix: value.prefix,
        }
    }
}

impl<T: StoragePrefix> TypedPrefixedStorageMut<'_, T> {
    pub fn borrow(&self) -> TypedPrefixedStorage<'_, T> {
        TypedPrefixedStorage {
            storage: self.storage,
            prefix: self.prefix.clone(),
            _data: PhantomData,
        }
    }
}
