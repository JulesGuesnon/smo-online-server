use bytes::{Buf, BufMut, Bytes, BytesMut};

#[derive(Debug)]
pub struct Position {
    x: f32,
    y: f32,
    z: f32,
}

impl Position {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn as_bytes(&self) -> Bytes {
        let mut bytes = BytesMut::new();

        bytes.put_f32(self.x);
        bytes.put_f32(self.y);
        bytes.put_f32(self.z);

        bytes.into()
    }

    pub fn from_bytes(mut bytes: Bytes) -> Self {
        Self {
            x: bytes.get_f32(),
            y: bytes.get_f32(),
            z: bytes.get_f32(),
        }
    }
}

#[derive(Debug)]
pub struct Quaternion {
    w: f32,
    x: f32,
    y: f32,
    z: f32,
}

impl Quaternion {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub fn as_bytes(&self) -> Bytes {
        let mut bytes = BytesMut::new();

        bytes.put_f32(self.x);
        bytes.put_f32(self.y);
        bytes.put_f32(self.z);
        bytes.put_f32(self.w);

        bytes.into()
    }

    pub fn from_bytes(mut bytes: Bytes) -> Self {
        Self {
            x: bytes.get_f32(),
            y: bytes.get_f32(),
            z: bytes.get_f32(),
            w: bytes.get_f32(),
        }
    }
}
