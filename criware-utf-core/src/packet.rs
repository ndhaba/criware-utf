use std::{
    alloc::{self, Layout},
    io::{Cursor, Read, Write},
    ops::{Deref, DerefMut},
};

use crate::{Error, IOErrorHelper, Result, Table};

mod cri_encryption;

fn aligned_vec(initial_size: usize, size: usize) -> Vec<u8> {
    let minimum_size = size.div_ceil(64) << 6;
    let layout = Layout::from_size_align(minimum_size, 64).expect("Invalid layout");
    unsafe {
        let ptr = alloc::alloc(layout);
        if ptr.is_null() {
            alloc::handle_alloc_error(layout);
        }
        Vec::from_raw_parts(ptr, initial_size, minimum_size)
    }
}
fn aligned_vec_empty() -> Vec<u8> {
    aligned_vec(0, 256)
}
fn aligned_vec_full(size: usize) -> Vec<u8> {
    aligned_vec(size, size)
}

/**
Packed, encryptable UTF table
 */
pub struct Packet<T: Table> {
    prefix: &'static [u8; 4],
    encrypted: bool,
    unknown_value: u32,
    table: T,
}

impl<T: Table> Packet<T> {
    /**
    Creates a new UTF table packet with the given prefix

    The table itself is initialized with `T::new()`
     */
    pub fn new(prefix: &'static [u8; 4]) -> Self {
        Self::from_table(T::new(), prefix)
    }

    /**
    Creates a new UTF table packet with the given prefix
     */
    pub fn from_table(table: T, prefix: &'static [u8; 4]) -> Self {
        Packet {
            prefix,
            encrypted: false,
            unknown_value: 0,
            table,
        }
    }

    /**
    Reads a UTF table packet from the given stream, verifying that it has
    the given 4-byte prefix.
     */
    pub fn read_packet(reader: &mut dyn Read, prefix: &'static [u8; 4]) -> Result<Self> {
        let mut header = [0u8; 16];
        reader.read_exact(&mut header).io("UTF packet header")?;
        if prefix != &header[0..4] {
            return Err(Error::WrongTableSchema);
        }
        let unknown_value = u32::from_le_bytes(header[4..8].try_into().unwrap());
        let table_size = u64::from_le_bytes(header[8..16].try_into().unwrap());
        if table_size < 32 {
            return Err(Error::MalformedHeader);
        }
        let mut table_data = aligned_vec_full(table_size as usize);
        let mut decrypted_table_data = aligned_vec_full(table_size as usize);
        reader
            .read_exact(table_data.as_mut_slice())
            .io("UTF table")?;
        if &table_data[0..4] == b"@UTF" {
            return Ok(Packet {
                prefix,
                encrypted: false,
                unknown_value,
                table: T::read(&mut Cursor::new(table_data))?,
            });
        }
        if !cri_encryption::can_decrypt(table_data.as_slice()) {
            return Err(Error::DecryptionError);
        }
        cri_encryption::decrypt(table_data.as_slice(), decrypted_table_data.as_mut_slice());
        if &decrypted_table_data[0..4] == b"@UTF" {
            return Ok(Packet {
                prefix,
                encrypted: true,
                unknown_value,
                table: T::read(&mut Cursor::new(decrypted_table_data))?,
            });
        }
        return Err(Error::DecryptionError);
    }

    /**
    Writes a UTF table packet to the given stream.
     */
    pub fn write_packet(&self, writer: &mut dyn Write) -> Result<()> {
        let mut table_buffer = Cursor::new(aligned_vec_empty());
        self.table.write(&mut table_buffer)?;
        let table_buffer = {
            let mut buffer = table_buffer.into_inner();
            if buffer.capacity() < buffer.len().div_ceil(64) << 6 {
                buffer.reserve_exact(64 - (buffer.len() & 63));
            }
            if self.encrypted {
                let mut new_buffer = aligned_vec_full(buffer.len());
                cri_encryption::encrypt(buffer.as_slice(), new_buffer.as_mut_slice());
                new_buffer
            } else {
                buffer
            }
        };
        writer.write_all(self.prefix).io("UTF packet header")?;
        writer
            .write_all(&u32::to_le_bytes(self.unknown_value))
            .io("UTF packet header")?;
        writer
            .write_all(&u64::to_le_bytes(table_buffer.len() as u64))
            .io("UTF packet header")?;
        writer
            .write_all(table_buffer.as_slice())
            .io("UTF packet table")?;
        Ok(())
    }

    /**
    Returns whether or not the table is encrypted
     */
    pub fn is_encrypted(&self) -> bool {
        self.encrypted
    }

    /**
    Disables encryption for this packet
     */
    pub fn disable_encryption(&mut self) {
        self.encrypted = false;
    }

    /**
    Enables encryption for this packet
     */
    pub fn enable_encryption(&mut self) {
        self.encrypted = true;
    }
}

impl<T: Table> Deref for Packet<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.table
    }
}

impl<T: Table> DerefMut for Packet<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.table
    }
}
