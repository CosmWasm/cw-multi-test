//! # Implementation of checksum generator

use cosmwasm_std::{Addr, Checksum};

/// This trait defines a method to calculate checksum based on
/// the creator's address and a unique code identifier.
pub trait ChecksumGenerator {
    /// Calculates the checksum for a given contract's code creator
    /// and code identifier. Returns a hexadecimal binary representation
    /// of the calculated checksum. There are no assumptions about
    /// the length of the calculated checksum.
    fn checksum(&self, creator: &Addr, code_id: u64) -> Checksum;
}

/// Default checksum generator implementation.
pub struct SimpleChecksumGenerator;

impl ChecksumGenerator for SimpleChecksumGenerator {
    /// Calculates the checksum based on code identifier. The resulting
    /// checksum is 32-byte length SHA2 digest.
    fn checksum(&self, _creator: &Addr, code_id: u64) -> Checksum {
        Checksum::generate(format!("contract code {}", code_id).as_bytes())
    }
}
