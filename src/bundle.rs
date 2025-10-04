use crate::errors::{JitoClientError, JitoClientResult};
use crate::grpc::{
    bundle::Bundle,
    packet::{Meta, Packet},
};
use solana_transaction::versioned::VersionedTransaction;

const TXNS_LIMIT: usize = 5;

impl Bundle {
    /// Creates a Bundle from a vec of transactions, to be sent via GRPC connection. Returns error if too many transactions.
    /// For each transaction, serialize the data and store it in a Packet, which then constitudes apart of a Bundle. Returns error if serialize fails.
    pub fn create(txns: &[VersionedTransaction]) -> JitoClientResult<Self> {
        if txns.len() > TXNS_LIMIT {
            return Err(JitoClientError::TooManyTxns);
        }

        Ok(Self {
            header: None,
            packets: Self::serialize(txns)?,
        })
    }

    // For each transaction, serialize the data and store it in a Packet, which then constitudes apart of a Bundle. Returns error if serialize fails
    fn serialize(txns: &[VersionedTransaction]) -> JitoClientResult<Vec<Packet>> {
        let mut packets = Vec::with_capacity(txns.len());
        for txn in txns {
            let data = bincode::serialize(&txn)?;
            let size = data.len() as u64;
            let packet = Packet {
                data,
                meta: Some(Meta {
                    size,
                    addr: "0.0.0.0".to_string(),
                    port: 0u32,
                    flags: None,
                    sender_stake: 0u64,
                }),
            };
            packets.push(packet);
        }
        Ok(packets)
    }
}
