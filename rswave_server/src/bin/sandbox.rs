use rswave_common::{
    packets::{NoveltyBeatsModeData, NoveltyBeatsModePacket, NoveltyModeData},
    rkyv::{
        Deserialize,
        check_archive,
        de::deserializers::AllocDeserializer,
        ser::{serializers::WriteSerializer, Serializer},
    },
};

fn main() {
    let packet = NoveltyBeatsModePacket::Data(NoveltyBeatsModeData {
        novelty: NoveltyModeData {
            value: 0.0,
            peak: 0.0,
        },
        beat: false,
    });

    let mut serializer = WriteSerializer::new(Vec::new());
    serializer.serialize_value(&packet).unwrap();
    let data = serializer.into_inner();
    println!("Deserialize buffer alignment: {}", (data.as_ptr() as usize).trailing_zeros());
    println!("({}) {:?}", data.len(), data);

    let archive = check_archive::<NoveltyBeatsModePacket>(&data, 0).unwrap();
    let deserialized = archive.deserialize(&mut AllocDeserializer).unwrap();

    println!("{:?}", deserialized);
}
