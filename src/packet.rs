use anyhow::Result;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use glam::{Quat, Vec3};
use std::ops::Range;
use std::str::from_utf8;
use uuid::Uuid;

const ID_RANGE: Range<usize> = 0..16;
const TYPE_RANGE: Range<usize> = 16..18;
const SIZE_RANGE: Range<usize> = 18..20;
pub const HEADER_SIZE: usize = 20;
const COSTUME_SIZE: usize = 0x20;
const STAGE_ID_SIZE: usize = 0x10;
const STAGE_SIZE: usize = 0x30;

trait AsBytes {
    fn as_bytes(&self) -> Bytes;
    fn from_bytes(bytes: Bytes) -> Self;
}

impl AsBytes for Vec3 {
    fn as_bytes(&self) -> Bytes {
        let mut bytes = BytesMut::new();

        bytes.put_f32(self.x);
        bytes.put_f32(self.y);
        bytes.put_f32(self.z);

        bytes.into()
    }

    fn from_bytes(mut bytes: Bytes) -> Self {
        Self {
            x: bytes.get_f32(),
            y: bytes.get_f32(),
            z: bytes.get_f32(),
        }
    }
}

impl AsBytes for Quat {
    fn as_bytes(&self) -> Bytes {
        let mut bytes = BytesMut::new();

        bytes.put_f32(self.x);
        bytes.put_f32(self.y);
        bytes.put_f32(self.z);
        bytes.put_f32(self.w);

        bytes.into()
    }

    fn from_bytes(mut bytes: Bytes) -> Self {
        Self {
            x: bytes.get_f32(),
            y: bytes.get_f32(),
            z: bytes.get_f32(),
            w: bytes.get_f32(),
        }
    }
}

trait AsByte {
    fn as_byte(&self) -> u8;
}

trait AsBool {
    fn as_bool(&self) -> bool;
}

impl AsByte for bool {
    fn as_byte(&self) -> u8 {
        if *self {
            1
        } else {
            0
        }
    }
}

impl AsBool for u8 {
    fn as_bool(&self) -> bool {
        if *self == 1 {
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub enum TagUpdate {
    Time,
    State,
}

impl TagUpdate {
    fn from_byte(byte: u8) -> Result<Self> {
        match byte {
            1 => Ok(Self::Time),
            2 => Ok(Self::State),
            b => Err(anyhow::anyhow!(
                "Invalid byte '{}', couldn't convert it to TagUpdate",
                b
            )),
        }
    }

    fn as_byte(&self) -> u8 {
        match self {
            Self::Time => 1,
            Self::State => 2,
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
            1 => Ok(Self::First),
            2 => Ok(Self::Reconnect),
            b => Err(anyhow::anyhow!(
                "Invalid byte '{}', couldn't convert it to ConnectionType",
                b
            )),
        }
    }

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
        cap_anim: String,
    },
    Game {
        is_2d: bool,
        scenario: u8,
        stage: String,
    },
    Tag {
        update_type: TagUpdate,
        is_it: bool,
        seconds: u8,
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
    fn serialize_string(string: String, size: usize) -> Bytes {
        let bytes = string.into_bytes();

        if bytes.len() > size {
            bytes.take(size).copy_to_bytes(size)
        } else {
            let padding: Vec<u8> = vec![0; size - bytes.len()];

            Bytes::from(bytes).chain(&padding[..]).copy_to_bytes(size)
        }
    }

    fn deserialize_string(bytes: Bytes) -> Result<String> {
        Ok(from_utf8(&bytes[..])?.trim_matches('\0').to_string())
    }

    fn serialize(&self) -> (Bytes, Bytes) {
        let mut body = BytesMut::new();

        let id = match self {
            Self::Unknown => 0i16,
            Self::Init { max_player } => {
                body.put_i16(max_player.clone());

                1
            }
            Self::Player {
                position,
                quaternion,
                animation_blend_weights,
                act,
                subact,
            } => {
                body.put(position.as_bytes());
                body.put(quaternion.as_bytes());
                body.put(Bytes::from(
                    animation_blend_weights
                        .into_iter()
                        .flat_map(|v| v.to_be_bytes())
                        .collect::<Vec<u8>>(),
                ));
                body.put_u16(act.clone());
                body.put_u16(subact.clone());

                2
            }
            Self::Cap {
                position,
                quaternion,
                cap_out,
                cap_anim,
            } => {
                body.put(position.as_bytes());
                body.put(quaternion.as_bytes());
                // C# encodes bool in 4 bytes
                body.put_u32(if *cap_out { 1 } else { 0 });
                body.put(Self::serialize_string(cap_anim.clone(), 0x30));

                3
            }
            Self::Game {
                is_2d,
                scenario,
                stage,
            } => {
                // C# encodes bool in 4 bytes, but here I don't get why it's 1 byte
                body.put_u8(is_2d.as_byte());
                body.put_u8(scenario.clone());
                body.put(Self::serialize_string(stage.clone(), 0x40));

                4
            }
            Self::Tag {
                update_type,
                is_it,
                seconds,
                minutes,
            } => {
                body.put_u8(update_type.as_byte());
                // C# encodes bool in 4 bytes, but here I don't get why it's 1 byte
                body.put_u8(is_it.as_byte());
                body.put_u8(seconds.clone());
                body.put_u16(minutes.clone());

                5
            }
            Self::Connect {
                type_,
                max_player,
                client,
            } => {
                body.put_u32(type_.as_u32());
                body.put_u16(max_player.clone());
                body.put(Self::serialize_string(client.clone(), COSTUME_SIZE));
                6
            }
            Self::Disconnect => 7,
            Self::Costume {
                body: body_name,
                cap,
            } => {
                body.put(Self::serialize_string(body_name.clone(), COSTUME_SIZE));
                body.put(Self::serialize_string(cap.clone(), COSTUME_SIZE));
                8
            }
            Self::Shine { id } => {
                body.put_i32(id.clone());
                9
            }
            Self::Capture { model } => {
                body.put(Self::serialize_string(model.clone(), COSTUME_SIZE));

                10
            }
            Self::ChangeStage {
                id,
                stage,
                scenario,
                sub_scenario,
            } => {
                body.put(Self::serialize_string(stage.clone(), STAGE_SIZE));
                body.put(Self::serialize_string(id.clone(), STAGE_ID_SIZE));
                body.put_i8(scenario.clone());
                body.put_u8(sub_scenario.clone());
                11
            }
        };

        let id = id.to_be_bytes().to_vec();

        (Bytes::from(id), body.into())
    }

    fn deserialize(id: i16, body: Bytes) -> Result<Self> {
        let packet = match id {
            1 => Self::Init {
                max_player: i16::from_be_bytes(body[..].try_into()?),
            },
            2 => Self::Player {
                position: Vec3::from_bytes(body.slice(0..12)),
                quaternion: Quat::from_bytes(body.slice(12..28)),
                animation_blend_weights: body
                    .slice(28..52)
                    .chunks(4)
                    .map(|mut chunk| chunk.get_f32())
                    .collect(),
                act: body.slice(52..54).get_u16(),
                subact: body.slice(54..56).get_u16(),
            },
            3 => Self::Cap {
                position: Vec3::from_bytes(body.slice(0..12)),
                quaternion: Quat::from_bytes(body.slice(12..28)),
                cap_out: if body.slice(28..32).get_u32() == 1 {
                    true
                } else {
                    false
                },
                cap_anim: Self::deserialize_string(body.slice(32..80))?,
            },
            4 => Self::Game {
                is_2d: body.slice(0..1).get_u8().as_bool(),
                scenario: body.slice(1..2).get_u8(),
                stage: Self::deserialize_string(body.slice(2..0x42))?,
            },
            5 => Self::Tag {
                update_type: TagUpdate::from_byte(body.slice(0..1).get_u8())?,
                is_it: body.slice(1..2).get_u8().as_bool(),
                seconds: body.slice(2..3).get_u8(),
                minutes: body.slice(3..5).get_u16(),
            },
            6 => Self::Connect {
                type_: ConnectionType::from_u32(body.slice(0..4).get_u32())?,
                max_player: body.slice(4..6).get_u16(),
                client: Self::deserialize_string(body.slice(6..COSTUME_SIZE))?,
            },
            7 => Self::Disconnect,
            8 => Self::Costume {
                body: Self::deserialize_string(body.slice(0..COSTUME_SIZE))?,
                cap: Self::deserialize_string(body.slice(COSTUME_SIZE..(COSTUME_SIZE * 2)))?,
            },
            9 => Self::Shine {
                id: body.slice(..).get_i32(),
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

        Ok(packet)
    }

    pub fn is_connect(&self) -> bool {
        match self {
            Self::Connect {
                type_: _,
                max_player: _,
                client: _,
            } => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Packet {
    pub id: Uuid,
    pub content: Content,
}

impl Packet {
    pub fn new(id: Uuid, content: Content) -> Self {
        Self { id, content }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let id: &[u8] = &self.id.into_bytes();

        let (type_, body) = self.content.serialize();

        let body = &body[..];

        let size = body.len() as i16;

        [id, &type_[..], &size.to_be_bytes(), body].concat()
    }
}

pub struct Header {
    pub id: Uuid,
    pub type_: i16,
    pub packet_size: usize,
}

impl Header {
    pub fn from_bytes(bytes: Bytes) -> Result<Self> {
        let b_id = bytes.slice(ID_RANGE)[..].try_into()?;
        let type_ = bytes.slice(TYPE_RANGE).get_i16();
        let packet_size = bytes.slice(SIZE_RANGE).get_i16() as usize;

        Ok(Self {
            id: Uuid::from_bytes(b_id),
            type_,
            packet_size,
        })
    }

    pub fn make_packet(&self, body: Bytes) -> Result<Packet> {
        Ok(Packet::new(
            self.id,
            Content::deserialize(self.type_, body)?,
        ))
    }
}
