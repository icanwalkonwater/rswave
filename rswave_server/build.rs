fn main() {
    if !cfg!(feature = "controller_ws2811") {
        panic!("You need to chose at least one LED controller !")
    }
}