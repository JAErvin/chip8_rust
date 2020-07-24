use crate::cpu;

use std::time::SystemTime;
//use std::thread;

use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use std::time::Duration;


const CPU_FREQ: u64 = 500; //adjust as desired. I saw this rate recommended
const TIMER_FREQ: u64 = 60;
const SPEC_FREQ: i32 = 44100;
const SCR_WIDTH: usize = 768;
const SCR_HEIGHT: usize = 1536;
const PADDING: usize = 1;
const PX_WIDTH: usize = 22; //+ 2*padding==24
const PX_HEIGHT: usize = 10; //+ 2*padding==12

const BG_COLOR: Color = Color::RGB(0, 0, 0);
const FG_COLOR: Color = Color::RGB(128, 128, 128);

fn select_key(keycode: Keycode) -> Option<usize> {
    return match keycode {
        Keycode::Num1 => Some(0x1),
        Keycode::Num2 => Some(0x2),
        Keycode::Num3 => Some(0x3),
        Keycode::Num4 => Some(0xC),
        Keycode::Q => Some(0x4),
        Keycode::W => Some(0x5),
        Keycode::E => Some(0x6),
        Keycode::R => Some(0xD),
        Keycode::A => Some(0x7),
        Keycode::S => Some(0x8),
        Keycode::D => Some(0x9),
        Keycode::F => Some(0xE),
        Keycode::Z => Some(0xA),
        Keycode::X => Some(0x0),
        Keycode::C => Some(0xB),
        Keycode::V => Some(0xF),
        _ => None,
    };
}

// source: https://docs.rs/sdl2/0.32.1/sdl2/audio/index.html
struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}
impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 {
                self.volume
            } else {
                -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

pub struct Emulator {
    cpu: cpu::CPU,
    canvas: WindowCanvas,
    event_pump: sdl2::EventPump,
    audio: AudioDevice<SquareWave>,
}

#[allow(dead_code)]
impl Emulator {
    pub fn new() -> Emulator {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        let window = video_subsystem
            .window("chip8", SCR_HEIGHT as u32, SCR_WIDTH as u32)
            .position_centered()
            .build()
            .unwrap();
        let canvas: WindowCanvas = window.into_canvas().build().unwrap();
        let event_pump = sdl_context.event_pump().unwrap();
        let spec = AudioSpecDesired {
            freq: Some(SPEC_FREQ),
            channels: Some(1),
            samples: None,
        };
        let audio = sdl_context
            .audio()
            .unwrap()
            .open_playback(None, &spec, |spec| SquareWave {
                phase_inc: 440.0 / spec.freq as f32,
                phase: 0.0,
                volume: 0.25,
            })
            .unwrap();
        Emulator {
            cpu: cpu::CPU::new(),
            canvas: canvas,
            event_pump: event_pump,
            audio: audio,
        }
    }

    fn draw(&mut self) {
        self.canvas.set_draw_color(BG_COLOR);
        self.canvas.clear();
        let mut rects: Vec<Rect> = vec![];
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
                    (PADDING + (x * (PADDING + PX_WIDTH + PADDING))) as i32,
                    (PADDING + (y * (PADDING + PX_HEIGHT + PADDING))) as i32,
                    (PADDING + PX_WIDTH + PADDING) as u32,
                    (PADDING + PX_HEIGHT + PADDING) as u32,
                ));
            }
        }

        self.canvas.fill_rects(&rects).unwrap();
        self.canvas.present();
    }

    fn read_input(&mut self) -> bool {
        //returns true if should quit
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => return true,
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => return true,
                Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    if let Some(key_num) = select_key(key) {
                        self.cpu.set_key(key_num, true);
                    }
                }
                Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    if let Some(key_num) = select_key(key) {
                        self.cpu.ignore_keypress = false;
                        self.cpu.set_key(key_num, false);
                    }
                }
                _ => {}
            }
        }
        return false;
    }

    pub fn run(&mut self, rom: &[u8; cpu::ROM_SIZE]) {
        //TODO: move all cycling into cpu; use callbacks for drawing, sound, input, etc
        self.cpu.load_rom(rom);
        self.draw(); //init
        let mut nanos_per_cycle = 1000000000 / CPU_FREQ;
        let mut cycles = 0;
        let mut time = SystemTime::now();
        let mut last_timer_update = time.clone();
        let mut sound_playing = false;
        loop {
            //do some time keeping
            let cycle_start = SystemTime::now();
            cycles = (cycles + 1) % CPU_FREQ;
            if cycles == 0 {
                //calculate seconds per CPU_FREQ
                let actual_time = match cycle_start.duration_since(time) {
                    Ok(t) => t.as_secs_f32(),
                    _ => 0.0,
                };
                println!(
                    "time for target ({}) cycles: {}    (sleep == {})",
                    CPU_FREQ, actual_time, nanos_per_cycle
                );
                time = cycle_start;
                // update nanos to try to more closely match
                let actual_nanos = (actual_time * 1000000000 as f32) as u64 / CPU_FREQ;
                let adjustment: i64 = ((1000000000 / CPU_FREQ) as i64 - actual_nanos as i64) / 2;
                println!("actual_nanos: {}\nadjustment: {}", actual_nanos, adjustment);
                nanos_per_cycle = (nanos_per_cycle as i64 + adjustment) as u64;
            }
            //update timers
            if last_timer_update.elapsed().unwrap() >= Duration::from_nanos(1000000000 / TIMER_FREQ)
            {
                self.cpu.update_timers();
                last_timer_update = cycle_start.clone();
            }

            if self.read_input() {
                return;
            };
            self.cpu.perform_cycle();
            if self.cpu.just_drew() {
                self.draw();
            }
            if sound_playing ^ self.cpu.should_play_sound() {
                sound_playing = !sound_playing;
                if sound_playing {
                    self.audio.resume();
                } else {
                    self.audio.pause();
                }
            }

            let sleep_time = Duration::from_nanos(nanos_per_cycle)
                .checked_sub(SystemTime::now().duration_since(cycle_start).unwrap());
            if let Some(pos_sleep_time) = sleep_time {
                std::thread::sleep(pos_sleep_time);
            }
        }
    }
}
