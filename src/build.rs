use fastrand::Rng;
use std::{env, fs::File, io::Write, path::Path};

fn main() -> std::io::Result<()> {
    write_randoms()?;
    write_logarithms()
}

fn create_output_file(name: &str) -> File {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join(name);

    File::create(dest_path).unwrap()
}

fn write_randoms() -> std::io::Result<()> {
    let mut f = create_output_file("zobrist.rs");

    let rng = Rng::new();
    rng.seed(16358476);

    let mut pieces = [[0; 64]; 12];
    for sq in 0..64 {
        for piece in 0..12 {
            pieces[piece][sq] = rng.u64(..);
        }
    }

    let mut castle = [0; 16];
    for c in 0..16 {
        castle[c] = rng.u64(..);
    }

    let mut ep = [0; 8];
    for e in 0..8 {
        ep[e] = rng.u64(..);
    }

    writeln!(f, "const PIECES: [[u64; 64]; 12] = {:?};", pieces)?;
    writeln!(f, "const SIDE: u64 = {:?};", rng.u64(..))?;
    writeln!(f, "const CASTLE: [u64; 16] = {:?};", castle)?;
    writeln!(f, "const EP: [u64; 8] = {:?};", ep)
}

fn write_logarithms() -> std::io::Result<()> {
    let mut f = create_output_file("logarithms.rs");

    let mut table = [0f32; 64];
    for i in 1..64 {
        table[i] = (i as f32).ln();
    }

    writeln!(f, "const LN: [f32; 64] = {:?};", table)
}
