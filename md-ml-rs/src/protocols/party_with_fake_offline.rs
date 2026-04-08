// By Boshi Yuan (Rust rewrite)

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

use crate::networking::Party;
use crate::share::{ShareElement, Spdz2kShare};

pub struct PartyWithFakeOffline<S: Spdz2kShare> {
    pub party: Party,
    global_key_shr: S::GlobalKeyType,
    reader: BufReader<File>,
}

impl<S: Spdz2kShare> PartyWithFakeOffline<S> {
    pub fn new(
        my_id: usize,
        num_parties: usize,
        port: usize,
        job_name: &str,
        fake_offline_dir: &str,
    ) -> Self {
        let party = Party::new(my_id, num_parties, port);

        let sep = if job_name.is_empty() { "" } else { "-" };
        let file_name = format!("{}{}party-{}.txt", job_name, sep, my_id);
        let path = PathBuf::from(fake_offline_dir).join(file_name);
        let file = File::open(&path).unwrap_or_else(|e| {
            panic!("Failed to open offline file {:?}: {}", path, e)
        });
        // 8MB buffer for fast sequential reading
        let mut reader = BufReader::with_capacity(8 * 1024 * 1024, file);

        // Read the MAC key (binary format)
        let global_key_shr = Self::read_one_value::<S::GlobalKeyType>(&mut reader);

        PartyWithFakeOffline {
            party,
            global_key_shr,
            reader,
        }
    }

    pub fn my_id(&self) -> usize {
        self.party.my_id()
    }

    pub fn bytes_sent(&self) -> u64 {
        self.party.bytes_sent()
    }

    pub fn global_key_shr(&self) -> S::GlobalKeyType {
        self.global_key_shr
    }

    pub fn read_shares(&mut self, num_elements: usize) -> Vec<S::SemiShrType> {
        let total_bytes = num_elements * S::SemiShrType::byte_size();
        let mut buf = vec![0u8; total_bytes];
        self.reader.read_exact(&mut buf).expect("Failed to read shares");
        S::SemiShrType::vec_from_bytes(&buf)
    }

    pub fn read_clear(&mut self, num_elements: usize) -> Vec<S::ClearType> {
        let total_bytes = num_elements * S::ClearType::byte_size();
        let mut buf = vec![0u8; total_bytes];
        self.reader.read_exact(&mut buf).expect("Failed to read clear values");
        S::ClearType::vec_from_bytes(&buf)
    }

    fn read_one_value<T: ShareElement>(reader: &mut BufReader<File>) -> T {
        let mut buf = vec![0u8; T::byte_size()];
        reader.read_exact(&mut buf).expect("Failed to read value");
        T::from_le_bytes(&buf)
    }
}
