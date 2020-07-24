mod cpu;
mod emulator;

use std::{
    env,
    fs
};


fn read_rom(path: String) -> [u8; cpu::ROM_SIZE] {
    let vector:Vec<u8> = fs::read(&path).unwrap();
    let mut rom:[u8; cpu::ROM_SIZE] = [0u8; cpu::ROM_SIZE];
    rom[0..vector.len()].copy_from_slice(&vector[0..]);
    rom
}


fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} path/to/rom", args[0]);
        return;
    }
    let rom_file = args[1].to_string();
    let mut emu = emulator::Emulator::new();
    emu.run(&read_rom(rom_file));

}
