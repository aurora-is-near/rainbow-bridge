use rlp::{Encodable, RlpStream};

#[derive(Debug, Clone)]
pub struct Bytes(pub Vec<u8>);

#[derive(Debug, Clone, PartialEq)]
pub struct U8(pub u8);

impl Encodable for U8 {
    fn rlp_append(&self, s: &mut RlpStream) {
        self.0.rlp_append(s);
    }
}

impl Encodable for Bytes {
    fn rlp_append(&self, s: &mut RlpStream) {
        self.0.rlp_append(s);
    }
}
