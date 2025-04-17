use crate::prefixed_storage::{prefixed, prefixed_read, PrefixedStorage, ReadonlyPrefixedStorage};
use cosmwasm_std::Storage;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

pub trait StoragePrefix {
    const PREFIX: &'static [u8];
}

pub struct TypedPrefixedStorage<'a, T>(ReadonlyPrefixedStorage<'a>, PhantomData<T>);

impl<'a, T: StoragePrefix> TypedPrefixedStorage<'a, T> {
    pub fn new(storage: &'a dyn Storage) -> Self {
        Self(prefixed_read(storage, T::PREFIX), PhantomData)
    }
}

impl<'a, T> Deref for TypedPrefixedStorage<'a, T> {
    type Target = ReadonlyPrefixedStorage<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct TypedPrefixedStorageMut<'a, T>(PrefixedStorage<'a>, PhantomData<T>);

impl<'a, T: StoragePrefix> TypedPrefixedStorageMut<'a, T> {
    pub fn new(storage: &'a mut dyn Storage) -> Self {
        Self(prefixed(storage, T::PREFIX), PhantomData)
    }
}

impl<'a, T> Deref for TypedPrefixedStorageMut<'a, T> {
    type Target = PrefixedStorage<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for TypedPrefixedStorageMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
