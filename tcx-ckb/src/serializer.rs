use byteorder::{ByteOrder, LittleEndian};
use std::collections::HashMap;
use tcx_chain::Result;

pub struct Serializer();

impl Serializer {
    fn calculate_offsets(element_lengths: &Vec<u32>) -> (u32, Vec<u32>) {
        let header_length = 4 + 4 * element_lengths.len() as u32;
        let mut offsets = vec![];
        let mut total = header_length;

        for i in 0..element_lengths.len() {
            offsets.push(total as u32);
            total = total + (element_lengths[i] as u32);
        }

        (total, offsets)
    }

    pub fn serialize_u32(value: u32) -> Vec<u8> {
        let mut buf = [0; 4];
        LittleEndian::write_u32(&mut buf, value);
        buf.to_vec()
    }

    pub fn serialize_u64(value: u64) -> Vec<u8> {
        let mut buf = [0; 8];
        LittleEndian::write_u64(&mut buf, value);
        buf.to_vec()
    }

    pub fn serialize_struct(values: &Vec<Vec<u8>>) -> Vec<u8> {
        let mut ret: Vec<u8> = vec![];

        for item in values.iter() {
            ret.extend(item);
        }

        ret
    }

    pub fn serialize_dynamic_vec(values: &Vec<Vec<u8>>) -> Vec<u8> {
        let mut body: Vec<u8> = vec![];
        let mut element_lengths: Vec<u32> = vec![];

        for item in values.iter() {
            element_lengths.push(item.len() as u32);
            body.extend(item);
        }

        let mut ret: Vec<u8> = vec![];

        let offsets = Serializer::calculate_offsets(&element_lengths);

        ret.extend(Serializer::serialize_u32(offsets.0));

        offsets.1.iter().for_each(|item| {
            ret.extend(Serializer::serialize_u32(*item));
        });

        ret.extend(body);

        ret
    }

    pub fn serialize_fixed_vec(values: &Vec<Vec<u8>>) -> Vec<u8> {
        let mut ret: Vec<u8> = vec![];
        let mut body: Vec<u8> = vec![];

        let mut total_size = 0 as u32;

        for item in values.iter() {
            total_size = total_size + item.len() as u32;

            body.extend(item);
        }

        ret.extend(Serializer::serialize_u32(total_size));
        ret.extend(body);

        ret
    }
}

#[cfg(test)]
mod tests {
    use crate::serializer::Serializer;

    #[test]
    fn serialize_struct() {
        let bytes = Serializer::serialize_struct(&vec![vec![0x11, 0x13], vec![0x20, 0x17, 0x9]]);
        assert_eq!(hex::encode(bytes), "1113201709");
    }

    #[test]
    fn serialize_fixed_vec() {
        let bytes =
            Serializer::serialize_fixed_vec(&vec![hex::decode("1234567890abcdef").unwrap()]);
        assert_eq!(hex::encode(bytes), "080000001234567890abcdef");
    }

    #[test]
    fn serialize_dynmaic_vec() {
        let bytes = Serializer::serialize_dynamic_vec(&vec![]);
        assert_eq!(hex::encode(bytes), "04000000");

        let bytes = Serializer::serialize_dynamic_vec(&vec![hex::decode("020000001234").unwrap()]);
        assert_eq!(hex::encode(bytes), "0e00000008000000020000001234");

        let bytes = Serializer::serialize_dynamic_vec(&vec![
            hex::decode("020000001234").unwrap(),
            hex::decode("00000000").unwrap(),
            hex::decode("020000000567").unwrap(),
            hex::decode("0100000089").unwrap(),
            hex::decode("03000000abcdef").unwrap(),
        ]);
        assert_eq!(hex::encode(bytes), "34000000180000001e00000022000000280000002d00000002000000123400000000020000000567010000008903000000abcdef");
    }
}
