use crate::MAGIC;
use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Archive, Serialize, Deserialize)]
#[archive(derive(CheckBytes))]
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
#[archive(derive(CheckBytes))]
pub enum DataMode {
    Novelty,
    NoveltyBeats,
}

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive(derive(CheckBytes))]
pub struct SetModePacket {
    pub mode: DataMode,
}

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive(derive(CheckBytes))]
pub enum NoveltyModePacket {
    Data(NoveltyModeData),
    Abort,
    Goodbye(GoodbyeData),
}

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive(derive(CheckBytes))]
pub struct NoveltyModeData {
    pub value: f64,
    pub peak: f64,
}

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive(derive(CheckBytes))]
pub enum NoveltyBeatsModePacket {
    Data(NoveltyBeatsModeData),
    Abort,
    Goodbye(GoodbyeData),
}

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive(derive(CheckBytes))]
pub struct NoveltyBeatsModeData {
    pub novelty: NoveltyModeData,
    pub beat: bool,
}

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive(derive(CheckBytes))]
pub struct GoodbyeData {
    pub magic: u8,
    pub force: bool,
}

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive(derive(CheckBytes))]
pub enum AckPacket {
    Ok,
    Quit,
    Abort,
}

#[cfg(test)]
mod tests {
    use crate::{
        packets::{
            GoodbyeData, NoveltyBeatsModeData, NoveltyBeatsModePacket, NoveltyModeData,
            NoveltyModePacket,
        },
        MAGIC,
    };
    use rkyv::{
        ser::{serializers::WriteSerializer, Serializer},
        Serialize,
    };
    use std::mem;

    fn serialize(t: &impl Serialize<WriteSerializer<Vec<u8>>>) -> Vec<u8> {
        let mut serializer = WriteSerializer::new(Vec::new());
        serializer.serialize_value(t).unwrap();
        serializer.into_inner()
    }

    #[test]
    fn test_serialize_novelty_beat_packet() {
        let packet_classic = NoveltyBeatsModePacket::Data(NoveltyBeatsModeData {
            novelty: NoveltyModeData {
                value: 0.0,
                peak: 0.0,
            },
            beat: false,
        });

        let packet_goodbye = NoveltyBeatsModePacket::Goodbye(GoodbyeData {
            magic: MAGIC,
            force: false,
        });

        let packet_abort = NoveltyBeatsModePacket::Abort;

        assert_eq!(mem::size_of::<NoveltyBeatsModePacket>(), 32);
        assert_eq!(
            serialize(&packet_classic).len(),
            mem::size_of::<NoveltyBeatsModePacket>()
        );
        assert_eq!(
            serialize(&packet_goodbye).len(),
            mem::size_of::<NoveltyBeatsModePacket>()
        );
        assert_eq!(
            serialize(&packet_abort).len(),
            mem::size_of::<NoveltyBeatsModePacket>()
        );
    }

    #[test]
    fn test_serialize_novelty_packet() {
        let packet_classic = NoveltyModePacket::Data(NoveltyModeData {
            value: 0.0,
            peak: 0.0,
        });

        let packet_goodbye = NoveltyModePacket::Goodbye(GoodbyeData {
            magic: MAGIC,
            force: false,
        });

        let packet_abort = NoveltyModePacket::Abort;

        assert_eq!(mem::size_of::<NoveltyModePacket>(), 24);
        assert_eq!(
            serialize(&packet_classic).len(),
            mem::size_of::<NoveltyModePacket>()
        );
        assert_eq!(
            serialize(&packet_goodbye).len(),
            mem::size_of::<NoveltyModePacket>()
        );
        assert_eq!(
            serialize(&packet_abort).len(),
            mem::size_of::<NoveltyModePacket>()
        );
    }
}
