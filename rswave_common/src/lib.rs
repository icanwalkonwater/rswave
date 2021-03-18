pub use int_enum::IntEnum;
pub use rkyv;

pub const MAGIC: u8 = 0x42;

pub mod packets;

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, IntEnum)]
pub enum LedMode {
    OnlyColor = 1,
    OnlyIntensity = 2,
    ColorAndIntensity = 3,
}
