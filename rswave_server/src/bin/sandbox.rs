use rswave_common::{packets::NoveltyBeatsModePacket, rkyv::Archived};
use std::mem;

fn main() {
    println!(
        "size {}, align {}",
        mem::size_of::<Archived<NoveltyBeatsModePacket>>(),
        mem::align_of::<Archived<NoveltyBeatsModePacket>>(),
    )
}
