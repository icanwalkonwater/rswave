use crate::MAGIC;
use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Archive, Serialize, Deserialize)]
#[archive(derive(Debug, CheckBytes))]
pub struct HelloPacket {
    pub magic: u8,
    pub random: u8,
}

impl Default for HelloPacket {
    fn default() -> Self {
        Self {
            magic: MAGIC,
            random: rand::random(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive(derive(Debug, CheckBytes))]
pub enum DataMode {
    Novelty,
    NoveltyBeats,
}

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive(derive(Debug, CheckBytes))]
pub struct SetModePacket {
    pub mode: DataMode,
}

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive(derive(Debug, CheckBytes))]
pub enum NoveltyModePacket {
    Data(NoveltyModeData),
    Abort,
    Goodbye(GoodbyeData),
}

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive(derive(Debug, CheckBytes))]
pub struct NoveltyModeData {
    pub value: f64,
    pub peak: f64,
}

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive(derive(Debug, CheckBytes))]
pub enum NoveltyBeatsModePacket {
    Data(NoveltyBeatsModeData),
    Abort,
    Goodbye(GoodbyeData),
}

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive(derive(Debug, CheckBytes))]
pub struct NoveltyBeatsModeData {
    pub novelty: NoveltyModeData,
    pub beat: bool,
}

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive(derive(Debug, CheckBytes))]
pub struct GoodbyeData {
    pub magic: u8,
    pub force: bool,
}

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive(derive(Debug, CheckBytes))]
pub enum AckPacket {
    Ok,
    Quit,
    Abort,
}
