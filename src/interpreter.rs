use std::time::Instant;

const INSTRUCTIONS_PER_SECOND: u32 = 700;
const SCREEN_WIDTH: usize = 64;
const SCREEN_HEIGHT: usize = 32;

const STACK_SIZE: usize = 48;
const MEMORY_SIZE: usize = 4096;
const TIMER_DECREMENT_FREQUENCY: f32 = 60.0;
const PC_START_ADDRESS: usize = 0x200;
const FONT_START_ADDRESS: usize = 0x50;
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
    pc: usize,
    i: usize,
    stack: Stack,
    memory: [u8; MEMORY_SIZE],
    registers: [u16; 16],
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
            registers: [0; 16],
            timers: Timers::new(),
            screen_buffer: [0; SCREEN_WIDTH * SCREEN_HEIGHT],
        }
    }

    pub fn load_program(&mut self, bytes: &[u8]) {
        for (i, byte) in bytes.iter().enumerate() {
            self.memory[PC_START_ADDRESS + i] = *byte;
        }
    }

    fn fetch_instruction(&mut self) -> u16 {
        let instruction = ((self.memory[self.pc] as u16) << 8) | self.memory[self.pc + 1] as u16;
        self.pc += 2;
        instruction
    }
}

#[derive(Debug, PartialEq)]
enum Instruction {
    NotImplemented,
    ClearScreen,
    Jump(usize),
    SetRegister(usize, u16),
    AddToRegister(usize, u16),
    SetI(u16),
    DrawSprite(usize, usize, u8),
}

impl Instruction {
    fn from_raw(bytes: u16) -> Self {
        match Self::nibble_left(bytes, 0) {
            0 => match bytes {
                0x00E0 => Self::ClearScreen,
                _ => Self::NotImplemented,
            },
            1 => Self::Jump((bytes & 0x0FFF) as usize),
            6 => Self::SetRegister(Self::nibble_left(bytes, 1) as usize, bytes & 0x00FF),
            7 => Self::AddToRegister(Self::nibble_left(bytes, 1) as usize, bytes & 0x00FF),
            0xA => Self::SetI(bytes & 0x0FFF),
            0xD => Self::DrawSprite(
                Self::nibble_left(bytes, 1) as usize,
                Self::nibble_left(bytes, 2) as usize,
                Self::nibble_left(bytes, 3),
            ),
            _ => Self::NotImplemented,
        }
    }

    fn nibble_left(bytes: u16, position: usize) -> u8 {
        assert!(position < 4);
        let mask = 0xF000 >> position * 4;
        let shift = 12 - position * 4;
        ((bytes & mask) >> shift) as u8
    }
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
        let amount = TIMER_DECREMENT_FREQUENCY * delta.as_secs_f32() + self.rounding_remainder;
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
        if self.delay_timer + self.sound_timer == 0 {
            self.rounding_remainder = 0.0;
        }

        self.last_update = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time::Duration};

    const PROGRAM: [u8; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

    #[test]
    fn load_program() {
        let mut interpreter = Interpreter::new();
        interpreter.load_program(&PROGRAM);

        for i in 0..PROGRAM.len() {
            assert_eq!(interpreter.memory[PC_START_ADDRESS + i], PROGRAM[i])
        }
    }

    #[test]
    fn fetch_instruction() {
        let mut interpreter = Interpreter::new();
        interpreter.load_program(&PROGRAM);

        assert_eq!(interpreter.fetch_instruction(), 1);
        assert_eq!(interpreter.fetch_instruction(), 0b0000001000000011);
        assert_eq!(interpreter.fetch_instruction(), 0b0000010000000101);
        assert_eq!(interpreter.fetch_instruction(), 0b0000011000000111);
        assert_eq!(interpreter.fetch_instruction(), 0b0000100000001001);
    }

    #[test]
    fn instruction_from_raw() {
        assert_eq!(Instruction::from_raw(0x00E0), Instruction::ClearScreen);
        assert_eq!(Instruction::from_raw(0x1FFF), Instruction::Jump(0x0FFF));
        assert_eq!(
            Instruction::from_raw(0x6502),
            Instruction::SetRegister(5, 2)
        );
        assert_eq!(
            Instruction::from_raw(0x70FF),
            Instruction::AddToRegister(0, 0xFF)
        );
        assert_eq!(Instruction::from_raw(0xAFFF), Instruction::SetI(0x0FFF));
        assert_eq!(
            Instruction::from_raw(0xD123),
            Instruction::DrawSprite(1, 2, 3)
        );
    }

    #[test]
    fn nibble() {
        let yummy = 0x1234;
        assert_eq!(Instruction::nibble_left(yummy, 0), 1);
        assert_eq!(Instruction::nibble_left(yummy, 1), 2);
        assert_eq!(Instruction::nibble_left(yummy, 2), 3);
        assert_eq!(Instruction::nibble_left(yummy, 3), 4);
    }

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
        lhs.abs_diff(rhs) <= max_deviation
    }
}
