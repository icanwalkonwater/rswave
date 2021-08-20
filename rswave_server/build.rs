fn main() {
    if cfg!(not(any(
        feature = "controller_ws2811",
        feature = "controller_gpio"
    ))) {
        panic!("You need to chose at least one LED controller !")
    }
}
