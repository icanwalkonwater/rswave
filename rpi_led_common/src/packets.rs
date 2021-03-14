use rkyv::{Archive, Serialize, Deserialize};

#[derive(Debug, Copy, Clone, Archive, Serialize, Deserialize)]
pub enum DataMode {
    Novelty,
    NoveltyBeats,
}

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
pub struct SetModePacket {
    mode: DataMode,
}

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
pub struct NoveltyModePacket {
    value: f32,
    peak: f32,
}

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
pub struct NoveltyBeatsModePacket {
    novelty: NoveltyModePacket,
    beat: bool,
}
