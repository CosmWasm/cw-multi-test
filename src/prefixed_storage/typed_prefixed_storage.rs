use crate::prefixed_storage::{prefixed, prefixed_read, PrefixedStorage, ReadonlyPrefixedStorage};
use cosmwasm_std::Storage;
use std::marker::PhantomData;

pub trait StoragePrefix {
    const PREFIX: &'static [u8];
}

pub struct TypedPrefixedStorage<'a, T>(ReadonlyPrefixedStorage<'a>, PhantomData<T>);

impl<'a, T: StoragePrefix> TypedPrefixedStorage<'a, T> {
    pub fn new(storage: &'a dyn Storage) -> Self {
        Self(prefixed_read(storage, T::PREFIX), PhantomData)
    }
}

impl<T> Storage for TypedPrefixedStorage<'_, T> {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.0.get(key)
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.0.set(key, value)
    }

    fn remove(&mut self, key: &[u8]) {
        self.0.remove(key)
    }
}

pub struct TypedPrefixedStorageMut<'a, T>(PrefixedStorage<'a>, PhantomData<T>);

impl<'a, T: StoragePrefix> TypedPrefixedStorageMut<'a, T> {
    pub fn new(storage: &'a mut dyn Storage) -> Self {
        Self(prefixed(storage, T::PREFIX), PhantomData)
    }
}

impl<T> Storage for TypedPrefixedStorageMut<'_, T> {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.0.get(key)
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.0.set(key, value)
    }

    fn remove(&mut self, key: &[u8]) {
        self.0.remove(key)
    }
}
