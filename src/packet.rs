use byteorder::{BigEndian, ByteOrder};

use pnet::packet::icmp::IcmpPacket;
//use pnet::packet::util::checksum;
use pnet_macros_support::packet::Packet;

use std::convert::From;
use std::fmt;
use std::io::Write;
use std::ptr;

#[derive(Clone)]
pub(crate) struct TunnelPacket {
    pub(crate) id: u16,
    pub(crate) seq: u16,
    pub(crate) raw_pack: [u8; 64],
}

impl TunnelPacket {
    pub(crate) fn new(id: u16, seq: u16, payload: [u8; 56]) -> TunnelPacket {
        let mut pack: [u8; 64] = [0; 64];

        BigEndian::write_u16(&mut pack[4..6], id);
        BigEndian::write_u16(&mut pack[6..8], seq);

        match (&mut pack[8..]).write(&payload) {
            Ok(_) => (),
            Err(_) => eprintln!("Error writing payload to packet at id {} seq {}", id, seq),
        };

        let checksum = TunnelPacket::calculate_checksum(&pack);
        BigEndian::write_u16(&mut pack[2..4], checksum);

        TunnelPacket {
            id: id,
            seq: seq,
            raw_pack: pack,
        }
    }

    #[allow(trivial_numeric_casts)]
    fn calculate_checksum<'t>(pack: &'t [u8]) -> u16 {
        // seems like we have to pick either safe or zero copy here
        let sum = pack.chunks(2).fold(0u32, |acc, word_slice| {
            let mut word: [u8; 2] = [0; 2];
            word.copy_from_slice(word_slice);
            acc.wrapping_add(u16::from_be_bytes(word) as u32);

            acc
        });
        let res = (sum >> 16) + (sum & 0xffff) + (sum >> 16);

        res as u16
    }
}

impl fmt::Debug for TunnelPacket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{ id: {}, seq: {}}}", self.id, self.seq)
    }
}

impl<'i> From<IcmpPacket<'i>> for TunnelPacket {
    fn from(item: IcmpPacket<'i>) -> Self {
        let mut idseq_buf = [0u8; 2];

        idseq_buf.copy_from_slice(&item.packet()[5..7]);
        let id = u16::from_be_bytes(idseq_buf);

        idseq_buf.copy_from_slice(&item.packet()[7..9]);
        let seq = u16::from_be_bytes(idseq_buf);

        let mut buf = [0u8; 56];
        buf.copy_from_slice(&item.payload()[4..]);

        TunnelPacket::new(id, seq, buf)
    }
}

impl ::pnet_macros_support::packet::Packet for TunnelPacket {
    #[inline]
    fn packet<'p>(&'p self) -> &'p [u8] {
        &self.raw_pack
    }

    #[inline]
    fn payload<'p>(&'p self) -> &'p [u8] {
        &self.raw_pack[8..]
    }
}

// turn a vec of bytes into a vec of 56-byte `TunnelPacket`s
// right now this requires a copy from the vec of data into arrays of
// a specific size
pub(crate) fn encode_packs(id: u16, payload: Vec<u8>) -> Vec<TunnelPacket> {
    // we'll need to subtract 8 bytes for the packet header from the standard
    // 64-byte echo packet size, leaving 56 bytes for the payload
    let chunk_size: usize = 56;
    let mut packs = Vec::new();

    let mut seq: u16 = 0;
    for chunk in payload.chunks(chunk_size) {
        let mut payload = [0u8; 56];
        if chunk.len() == 56 {
            payload.copy_from_slice(chunk);
        } else {
            // no comfy, idiomatic way to do the same thing as `copy_from_slice`
            // so we'll just use the unsafe equivalent to C's `memmove`
            //
            // this shouldn't pose any problems, because the length of the chunk
            // should always be 56 bytes or fewer
            unsafe { ptr::copy(chunk.as_ptr(), payload.as_mut_ptr(), chunk.len()); }
        }

        let pack = TunnelPacket::new(id, seq, payload);
        packs.push(pack);

        seq += 1;
    }

    packs
}

pub(crate) fn decode_packs(packs: [u8; 336]) -> Vec<TunnelPacket> {
    let chunk_size: usize = 84;

    let mut chunk_buf = [0u8; 56];
    let mut decoded_packs = Vec::new();

    for chunk in packs.chunks(chunk_size) {
        let id = BigEndian::read_u16(&chunk[4..6]);
        let seq = BigEndian::read_u16(&chunk[6..8]);
        chunk_buf.copy_from_slice(&chunk[28..]);

        let pack = TunnelPacket::new(id, seq, chunk_buf);
        decoded_packs.push(pack);
    }

    decoded_packs
}
