// Components vendored in from other projects, adapted as needed.
//
// These are kept byte-identical to upstream where possible so they stay
// diffable against the source they came from. Each `mod` below carries
// `#[rustfmt::skip]` so `cargo fmt` leaves the file alone — add it to every
// new module here too, since the attribute applies per-item.

#[rustfmt::skip]
pub mod text_input;
