//! Wrap a RISC0 user ELF into the risc0 ProgramBinary format that
//! spel/lgs consume, and print the derived program ID.
use std::{env, fs, path::PathBuf};
use risc0_binfmt::ProgramBinary;
use risc0_zkos_v1compat::V1COMPAT_ELF;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: mk-program-binary <user.elf> <output.bin>");
        std::process::exit(1);
    }
    let user_elf = fs::read(&args[1]).expect("read user elf");
    let kernel_elf = V1COMPAT_ELF.to_vec();
    let binary = ProgramBinary::new(&user_elf, &kernel_elf);
    let encoded = binary.encode();
    let out = PathBuf::from(&args[2]);
    fs::write(&out, &encoded).expect("write output");
    println!("Wrote {} bytes to {}", encoded.len(), out.display());
    let check = ProgramBinary::decode(&encoded).expect("decode check failed");
    let id = check.compute_image_id().expect("compute image id");
    let id_bytes: Vec<u8> = id.as_words().iter().flat_map(|w| w.to_le_bytes()).collect();
    let hex: String = id_bytes.iter().map(|b| format!("{:02x}", b)).collect();
    println!("ProgramId: {}", hex);
}
