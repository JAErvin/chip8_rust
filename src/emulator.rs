use crate::cpu;

use std::time::SystemTime;
use std::{thread};

//use sdl2::pixels::Color;
//use sdl2::event::Event;
//use sdl2::keyboard::Keycode;
//use sdl2::render::WindowCanvas;
//use sdl2::rect::Rect;
//use std::time::Duration;

const FPS: u8 = 60;
const NANOS_PER_CYCLE: u64 = 1000000000 / 60;
const SCR_WIDTH: usize = 768;
const SCR_HEIGHT: usize = 1536;
const PADDING: usize = 1;
const PX_WIDTH: usize = 22; //+ 2*padding==24
const PX_HEIGHT: usize = 10; //+ 2*padding==12


pub struct Emulator {
    cpu: cpu::CPU,
}

pub fn new() -> Emulator {
    Emulator{
        cpu: cpu::new()
    }

}

#[allow(dead_code)]
impl Emulator {
    fn draw(&self) {}

    pub fn run(&mut self, rom: &[u8; cpu::ROM_SIZE]) {
        self.cpu.load_rom(rom);
        loop {
            let cycle_start = SystemTime::now();
            self.cpu.perform_cycle();
            if self.cpu.just_drew() {
                self.draw();
            }
            let sleep_time = SystemTime::now().duration_since(cycle_start).unwrap();
            thread::sleep(sleep_time);
        }
    }
}
