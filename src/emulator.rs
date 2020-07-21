use crate::cpu;

use std::time::SystemTime;
use std::{thread};

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::WindowCanvas;
use sdl2::rect::Rect;
use std::time::Duration;

const FPS: u8 = 60;
const NANOS_PER_CYCLE: u64 = 1000000000 / 60;
const SCR_WIDTH: usize = 768;
const SCR_HEIGHT: usize = 1536;
const PADDING: usize = 1;
const PX_WIDTH: usize = 22; //+ 2*padding==24
const PX_HEIGHT: usize = 10; //+ 2*padding==12

const BG_COLOR: Color = Color::RGB(0,0,0);
const FG_COLOR: Color = Color::RGB(128,128,128);


pub struct Emulator {
    cpu: cpu::CPU,
    sdl_context: sdl2::Sdl,
    video_subsystem: sdl2::VideoSubsystem,
    //window: sdl2::video::Window,
    canvas: WindowCanvas,
    event_pump: sdl2::EventPump,
}

pub fn new() -> Emulator {
    let mut sdl_context = sdl2::init().unwrap();
    let mut video_subsystem = sdl_context.video().unwrap();
    let mut window = video_subsystem.window("chip8", SCR_HEIGHT as u32, SCR_WIDTH as u32)
        .position_centered()
        .build()
        .unwrap();
    let mut canvas: WindowCanvas = window.into_canvas()
        .build()
        .unwrap();
    let mut event_pump =sdl_context.event_pump().unwrap();
    Emulator{
        cpu: cpu::new(),
        sdl_context: sdl_context,
        video_subsystem: video_subsystem,
        //window: window,
        canvas: canvas,
        event_pump: event_pump,
    }

}

#[allow(dead_code)]
impl Emulator {
    fn draw(&mut self) {
        self.canvas.set_draw_color(BG_COLOR);
        self.canvas.clear();
        let mut rects: Vec<Rect> = vec!();
        self.canvas.set_draw_color(FG_COLOR);
        let rects_to_draw = self.cpu.get_gfx();
        for i in 0..rects_to_draw.len() {
            //print!("{}", if rects_to_draw[i] {"T "} else { "F "});
            //if i == cpu::GFX_COLS { println!(""); }
            if rects_to_draw[i] {
                //println!("PIXEL!!");
                let (x, y) = cpu::index_to_coords(i as u16);
                // actual width = 2x padding + px_width
                // actual height = 2x padding + px_height
                rects.push(Rect::new(
                        (PADDING + (x * (PADDING + PX_WIDTH  + PADDING))) as i32,
                        (PADDING + (y * (PADDING + PX_HEIGHT + PADDING))) as i32,
                        (PADDING + PX_WIDTH  + PADDING) as u32,
                        (PADDING + PX_HEIGHT + PADDING) as u32,
                        ));
            }
        }

        self.canvas.fill_rects(&rects).unwrap();
        self.canvas.present();
    }

    pub fn run(&mut self, rom: &[u8; cpu::ROM_SIZE]) {
        self.cpu.load_rom(rom);
        self.draw(); //init
        loop {
            let cycle_start = SystemTime::now();
            self.cpu.perform_cycle();
            self.draw(); //TODO: Delete
            if self.cpu.just_drew() {
                //println!("just drew");
                //self.draw();
            }
            let sleep_time = Duration::from_nanos(NANOS_PER_CYCLE)
                .checked_sub(SystemTime::now()
                    .duration_since(cycle_start)
                    .unwrap()
                );
            if let Some(pos_sleep_time) = sleep_time {
                std::thread::sleep(pos_sleep_time);
            }
            //std::thread::sleep(Duration::from_millis(2000));
        }
    }
}
