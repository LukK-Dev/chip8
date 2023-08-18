#![allow(dead_code)]

use std::time::Instant;

fn main() {
    println!("Hello, world!");
}

const STACK_SIZE: usize = 48;
const MEMORY_SIZE: usize = 4096;
const SCREEN_WIDTH: usize = 64;
const SCREEN_HEIGHT: usize = 32;
const TIMER_DECREMENT_FREQUENCY: u32 = 60;
const PC_START_ADDRESS: u16 = 0x200;
const FONT_START_ADDRESS: u16 = 0x50;
const FONT: [u8; 16 * 5] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

struct Interpreter {
    pc: u16,
    i: u16,
    stack: Stack,
    memory: [u8; MEMORY_SIZE],
    registers: Registers,
    timers: Timers,
    screen_buffer: [u8; SCREEN_WIDTH * SCREEN_HEIGHT],
}

impl Interpreter {
    pub fn new() -> Self {
        let mut memory = [0; MEMORY_SIZE];
        for i in 0..FONT.len() {
            memory[FONT_START_ADDRESS as usize + i] = FONT[i];
        }

        Self {
            pc: PC_START_ADDRESS,
            i: 0,
            stack: Stack::new(),
            memory,
            registers: Registers::default(),
            timers: Timers::new(),
            screen_buffer: [0; SCREEN_WIDTH * SCREEN_HEIGHT],
        }
    }
}

#[derive(Debug, Default)]
struct Registers {
    pub v0: u16,
    pub v1: u16,
    pub v2: u16,
    pub v3: u16,
    pub v4: u16,
    pub v5: u16,
    pub v6: u16,
    pub v7: u16,
    pub v8: u16,
    pub v9: u16,
    pub va: u16,
    pub vb: u16,
    pub vc: u16,
    pub vd: u16,
    pub ve: u16,
    pub vf: u16,
}

struct Stack {
    data: [u8; STACK_SIZE],
    position: usize,
}

impl Stack {
    pub fn new() -> Self {
        Self {
            data: [0; STACK_SIZE],
            position: 0,
        }
    }

    pub fn push(&mut self, byte: u8) {
        if self.position > STACK_SIZE - 1 {
            panic!("stack overflow")
        }

        self.data[self.position] = byte;
        self.position += 1;
    }

    pub fn pop(&mut self) -> Option<u8> {
        if self.position == 0 {
            return None;
        }

        self.position -= 1;
        Some(self.data[self.position])
    }
}

struct Timers {
    pub delay_timer: u8,
    pub sound_timer: u8,
    last_update: Instant,
    rounding_remainder: f32,
}

impl Timers {
    pub fn new() -> Self {
        Self {
            delay_timer: 0,
            sound_timer: 0,
            last_update: Instant::now(),
            rounding_remainder: 0.0,
        }
    }

    pub fn decrement_timers(&mut self) {
        let now = Instant::now();
        let delta = now - self.last_update;
        let amount =
            TIMER_DECREMENT_FREQUENCY as f32 * delta.as_secs_f32() + self.rounding_remainder;
        self.rounding_remainder = amount - amount.floor();
        let amount = amount.floor() as u8;

        if self.delay_timer > amount {
            self.delay_timer -= amount;
        } else {
            self.delay_timer = 0;
        }
        if self.sound_timer > amount {
            self.sound_timer -= amount;
        } else {
            self.sound_timer = 0;
        }

        self.last_update = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time::Duration};

    #[test]
    fn stack_pushing_and_popping() {
        let mut stack = Stack::new();
        stack.push(10);
        stack.push(20);
        assert_eq!(stack.pop(), Some(20));
        stack.push(30);
        assert_eq!(stack.pop(), Some(30));
        assert_eq!(stack.pop(), Some(10));
        assert_eq!(stack.pop(), None);
    }

    #[test]
    fn decrement_timers() {
        let mut timers = Timers::new();
        timers.delay_timer = 90;
        timers.sound_timer = 30;
        thread::sleep(Duration::from_millis(500));
        timers.decrement_timers();
        assert!(approx_equal_u8(timers.delay_timer, 60, 1));
        assert!(approx_equal_u8(timers.sound_timer, 0, 1));
        thread::sleep(Duration::from_millis(500));
        timers.decrement_timers();
        timers.decrement_timers();
        assert!(approx_equal_u8(timers.delay_timer, 30, 1));
        assert!(approx_equal_u8(timers.sound_timer, 0, 1));
    }

    fn approx_equal_u8(lhs: u8, rhs: u8, max_deviation: u8) -> bool {
        let (lhs, rhs) = if lhs > rhs { (lhs, rhs) } else { (rhs, lhs) };
        lhs - rhs <= max_deviation
    }
}
