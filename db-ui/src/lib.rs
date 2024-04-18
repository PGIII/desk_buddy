use rust_embed::RustEmbed;

pub mod pages;
#[derive(RustEmbed)]
#[folder = "images/bmp/40/"]
struct Icons40;

#[derive(RustEmbed)]
#[folder = "images/bmp/20/"]
struct Icons20;

