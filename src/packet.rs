use std::ops::Range;
use std::str::from_utf8;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use color_eyre::eyre::eyre;
use color_eyre::Result;
use glam::{Quat, Vec3};
use uuid::Uuid;

const ID_RANGE: Range<usize> = 0..16;
const TYPE_RANGE: Range<usize> = 16..18;
const SIZE_RANGE: Range<usize> = 18..20;
pub const HEADER_SIZE: usize = 20;
const COSTUME_SIZE: usize = 0x20;
const STAGE_ID_SIZE: usize = 0x10;
const STAGE_SIZE: usize = 0x30;

trait AsBytes {
    fn write_bytes(&self, bytes: &mut BytesMut);
    fn from_bytes(bytes: Bytes) -> Self;
}

impl AsBytes for Vec3 {
    fn write_bytes(&self, bytes: &mut BytesMut) {
        bytes.put_f32_le(self.x);
        bytes.put_f32_le(self.y);
        bytes.put_f32_le(self.z);
    }

    fn from_bytes(mut bytes: Bytes) -> Self {
        Self {
            x: bytes.get_f32_le(),
            y: bytes.get_f32_le(),
            z: bytes.get_f32_le(),
        }
    }
}

impl AsBytes for Quat {
    fn write_bytes(&self, bytes: &mut BytesMut) {
        bytes.put_f32_le(self.x);
        bytes.put_f32_le(self.y);
        bytes.put_f32_le(self.z);
        bytes.put_f32_le(self.w);
    }

    fn from_bytes(mut bytes: Bytes) -> Self {
        Quat::from_xyzw(
            bytes.get_f32_le(),
            bytes.get_f32_le(),
            bytes.get_f32_le(),
            bytes.get_f32_le(),
        )
    }
}

trait AsByte {
    fn as_byte(&self) -> u8;
}

trait AsBool {
    fn as_bool(&self) -> bool;
}

impl AsByte for bool {
    #[inline(always)]
    fn as_byte(&self) -> u8 {
        if *self {
            1
        } else {
            0
        }
    }
}

impl AsBool for u8 {
    #[inline(always)]
    fn as_bool(&self) -> bool {
        *self == 1
    }
}

#[derive(Debug, Clone)]
pub enum TagUpdate {
    Time,
    State,
}

impl TagUpdate {
    pub fn as_byte(&self) -> u8 {
        match self {
            Self::Time => 0x1,
            Self::State => 0x2,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConnectionType {
    First,
    Reconnect,
}

impl ConnectionType {
    fn from_u32(byte: u32) -> Result<Self> {
        match byte {
            0 => Ok(Self::First),
            1 => Ok(Self::Reconnect),
            b => Err(eyre!(
                "Invalid byte '{}', couldn't convert it to ConnectionType",
                b
            )),
        }
    }

    #[inline]
    fn as_u32(&self) -> u32 {
        match self {
            Self::First => 1,
            Self::Reconnect => 2,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Content {
    Unknown,
    Init {
        max_player: i16,
    },
    Player {
        position: Vec3,
        quaternion: Quat,
        animation_blend_weights: Vec<f32>,
        act: u16,
        subact: u16,
    },
    Cap {
        position: Vec3,
        quaternion: Quat,
        cap_out: bool,
        cap_anim: Vec<u8>,
    },
    Game {
        is_2d: bool,
        scenario: u8,
        stage: String,
    },
    Tag {
        // It's a bitfield
        update_type: u8,
        is_it: bool,
        seconds: u16,
        minutes: u16,
    },
    Connect {
        type_: ConnectionType,
        max_player: u16,
        client: String,
    },
    Disconnect,
    Costume {
        body: String,
        cap: String,
    },
    Shine {
        id: i32,
    },
    Capture {
        model: String,
    },
    ChangeStage {
        id: String,
        stage: String,
        scenario: i8,
        sub_scenario: u8,
    },
}

impl Content {
    fn serialize_string(string: String, size: usize, buf: &mut BytesMut) {
        let bytes = string.into_bytes();

        if bytes.len() > size {
            buf.put(bytes.take(size));
        } else {
            let padding: Vec<u8> = vec![0; size - bytes.len()];

            buf.put(&bytes[..]);
            buf.put(&padding[..]);
        }
    }

    fn deserialize_string(bytes: Bytes) -> Result<String> {
        Ok(from_utf8(&bytes[..])?.trim_matches('\0').to_owned())
    }

    fn serialize(&self) -> (Bytes, Bytes) {
        let mut body = BytesMut::with_capacity(64);
        match &self {
            Self::Player {
                position: _,
                quaternion: _,
                animation_blend_weights: _,
                act: _,
                subact: _,
            } => (),

            Self::Cap {
                position: _,
                quaternion: _,
                cap_out: _,
                cap_anim: _,
            } => (),

            _ => {
                tracing::trace!(packet = ?self, "outgoing");
            }
        }

        let id = match self {
            Self::Unknown => 0i16,
            Self::Init { max_player } => {
                body.put_i16_le(*max_player);

                1
            }
            Self::Player {
                position,
                quaternion,
                animation_blend_weights,
                act,
                subact,
            } => {
                position.write_bytes(&mut body);
                quaternion.write_bytes(&mut body);
                for f in animation_blend_weights {
                    body.put_f32_le(*f);
                }
                body.put_u16_le(*act);
                body.put_u16_le(*subact);

                2
            }
            Self::Cap {
                position,
                quaternion,
                cap_out,
                cap_anim,
            } => {
                position.write_bytes(&mut body);
                quaternion.write_bytes(&mut body);
                body.put_u8(cap_out.as_byte());
                // body.put(Self::serialize_string(cap_anim.clone(), 0x30));
                body.put(&cap_anim[..]);

                3
            }
            Self::Game {
                is_2d,
                scenario,
                stage,
            } => {
                body.put_u8(is_2d.as_byte());
                body.put_u8(*scenario);
                Self::serialize_string(stage.clone(), 0x40, &mut body);

                4
            }
            Self::Tag {
                update_type,
                is_it,
                seconds,
                minutes,
            } => {
                body.put_u8(*update_type);
                body.put_u8(is_it.as_byte());
                body.put_u16_le(*seconds);
                body.put_u16_le(*minutes);

                5
            }
            Self::Connect {
                type_,
                max_player,
                client,
            } => {
                body.put_u32_le(type_.as_u32());
                body.put_u16_le(*max_player);
                Self::serialize_string(client.clone(), COSTUME_SIZE, &mut body);
                6
            }
            Self::Disconnect => 7,
            Self::Costume {
                body: body_name,
                cap,
            } => {
                Self::serialize_string(body_name.clone(), COSTUME_SIZE, &mut body);
                Self::serialize_string(cap.clone(), COSTUME_SIZE, &mut body);
                8
            }
            Self::Shine { id } => {
                body.put_i32_le(*id);
                9
            }
            Self::Capture { model } => {
                Self::serialize_string(model.clone(), COSTUME_SIZE, &mut body);

                10
            }
            Self::ChangeStage {
                id,
                stage,
                scenario,
                sub_scenario,
            } => {
                Self::serialize_string(stage.clone(), STAGE_SIZE, &mut body);
                Self::serialize_string(id.clone(), STAGE_ID_SIZE, &mut body);
                body.put_i8(*scenario);
                body.put_u8(*sub_scenario);
                11
            }
        };

        let id = id.to_le_bytes().to_vec();

        (Bytes::from(id), body.into())
    }

    fn deserialize(id: i16, body: Bytes) -> Result<Self> {
        let packet = match id {
            1 => Self::Init {
                max_player: i16::from_le_bytes(body[..].try_into()?),
            },
            2 => Self::Player {
                position: Vec3::from_bytes(body.slice(0..12)),
                quaternion: Quat::from_bytes(body.slice(12..28)),
                animation_blend_weights: body
                    .slice(28..52)
                    .chunks(4)
                    .map(|mut chunk| chunk.get_f32_le())
                    .collect(),
                act: body.slice(52..54).get_u16_le(),
                subact: body.slice(54..56).get_u16_le(),
            },
            3 => Self::Cap {
                position: Vec3::from_bytes(body.slice(0..12)),
                quaternion: Quat::from_bytes(body.slice(12..28)),
                cap_out: body.slice(28..29).get_u8().as_bool(),
                cap_anim: body.slice(29..(29 + 0x30)).to_vec(),
            },
            4 => Self::Game {
                is_2d: body.slice(0..1).get_u8().as_bool(),
                scenario: body.slice(1..2).get_u8(),
                stage: Self::deserialize_string(body.slice(2..0x42))?,
            },
            5 => {
                if body.len() == 5 {
                    Self::Tag {
                        update_type: body.slice(0..1).get_u8(),
                        is_it: body.slice(1..2).get_u8().as_bool(),
                        seconds: u16::from(body.slice(2..3).get_u8()),
                        minutes: body.slice(3..5).get_u16_le(),
                    }
                } else {
                    Self::Tag {
                        update_type: body.slice(0..1).get_u8(),
                        is_it: body.slice(1..2).get_u8().as_bool(),
                        seconds: body.slice(2..4).get_u16_le(),
                        minutes: body.slice(4..6).get_u16_le(),
                    }
                }
            }
            6 => Self::Connect {
                type_: ConnectionType::from_u32(body.slice(0..4).get_u32_le())?,
                max_player: body.slice(4..6).get_u16_le(),
                client: Self::deserialize_string(body.slice(6..COSTUME_SIZE))?,
            },
            7 => Self::Disconnect,
            8 => Self::Costume {
                body: Self::deserialize_string(body.slice(0..COSTUME_SIZE))?,
                cap: Self::deserialize_string(body.slice(COSTUME_SIZE..(COSTUME_SIZE * 2)))?,
            },
            9 => Self::Shine {
                id: body.slice(..4).get_i32_le(),
            },
            10 => Self::Capture {
                model: Self::deserialize_string(body.slice(0..COSTUME_SIZE))?,
            },
            11 => Self::ChangeStage {
                stage: Self::deserialize_string(body.slice(0..STAGE_SIZE))?,
                id: Self::deserialize_string(body.slice(STAGE_SIZE..(STAGE_SIZE + STAGE_ID_SIZE)))?,
                scenario: body
                    .slice((STAGE_SIZE + STAGE_ID_SIZE)..(STAGE_SIZE + STAGE_ID_SIZE + 1))
                    .get_i8(),
                sub_scenario: body
                    .slice((STAGE_SIZE + STAGE_ID_SIZE + 1)..(STAGE_SIZE + STAGE_ID_SIZE + 2))
                    .get_u8(),
            },
            _ => Self::Unknown,
        };

        match &packet {
            Self::Player {
                position: _,
                quaternion: _,
                animation_blend_weights: _,
                act: _,
                subact: _,
            } => (),

            Self::Cap {
                position: _,
                quaternion: _,
                cap_out: _,
                cap_anim: _,
            } => (),

            _ => {
                tracing::trace!(?packet, "incoming");
            }
        }

        Ok(packet)
    }

    #[inline]
    pub fn is_connect(&self) -> bool {
        matches!(
            self,
            Self::Connect {
                type_: _,
                max_player: _,
                client: _,
            }
        )
    }

    #[inline]
    pub fn is_disconnect(&self) -> bool {
        matches!(self, Self::Disconnect)
    }
}

#[derive(Debug, Clone)]
pub struct Packet {
    pub id: Uuid,
    pub content: Content,
}

impl Packet {
    #[inline]
    pub fn new(id: Uuid, content: Content) -> Self {
        Self { id, content }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let id: &[u8] = &self.id.into_bytes();

        let (type_, body) = self.content.serialize();

        let body = &body[..];

        let size = body.len() as i16;

        [id, &type_[..], &size.to_le_bytes(), body].concat()
    }
}

#[derive(Debug)]
pub struct Header {
    pub id: Uuid,
    pub type_: i16,
    pub packet_size: usize,
}

impl Header {
    pub fn from_bytes(bytes: Bytes) -> Result<Self> {
        let b_id = bytes.slice(ID_RANGE)[..].try_into()?;
        let type_ = bytes.slice(TYPE_RANGE).get_i16_le();
        let packet_size = bytes.slice(SIZE_RANGE).get_i16_le() as usize;

        Ok(Self {
            id: Uuid::from_bytes(b_id),
            type_,
            packet_size,
        })
    }

    pub fn make_packet(&self, body: Bytes) -> Result<Packet> {
        let packet = Packet::new(self.id, Content::deserialize(self.type_, body)?);

        Ok(packet)
    }
}
