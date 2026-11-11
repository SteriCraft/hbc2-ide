//! The library keeps interrupts off for one mailbox command at a time, not
//! for a whole call (screen.has): the longest stretch with the I flag off,
//! measured instruction by instruction in long screen calls.
//! Needs the compiler tools (compiler/build.cmd).

use hbc2_cc::{build_library, build_program, report};
use hbc2_core::{Flag, Machine, MachineConfig, StopReason};

const MAIN: &str = "#include <hbc2.h>
static char text[SCREEN_COLUMNS * SCREEN_ROWS];
void mark(void);
int main(void)
{
    unsigned i;
    for (i = 0; i < sizeof text; i++)
        text[i] = 'a' + i % 26;
    interrupts_on();
    mark();
    screen_write(text, sizeof text, 0, 0, COLORS(WHITE, BLUE));
    mark();
    screen_set_bg(GREEN);
    mark();
    screen_clear(COLORS(WHITE, BLACK));
    screen_line(0, 0, 255, 191, RED);
    screen_set_txt(BRIGHT_YELLOW);
    mark();
    return 0;
}
";

/// The longest stretch of cycles with interrupts off, and the cycles in
/// all, from one mark() to the next.
fn measure(m: &mut Machine, mark: u16) -> (u64, u64) {
    m.step(); // past the breakpoint
    let start = m.ticks();
    let (mut longest, mut off_since) = (0, None);
    while m.cpu.pc != mark {
        let before = m.ticks();
        m.step();
        if m.cpu.flag(Flag::Interrupt) {
            if let Some(since) = off_since.take() {
                longest = longest.max(before - since);
            }
        } else if off_since.is_none() {
            off_since = Some(before);
        }
        assert!(m.ticks() - start < 5_000_000, "mark() not reached");
    }
    (longest, m.ticks() - start)
}

#[test]
fn interrupts_are_off_for_one_command_at_a_time() {
    let dir = std::env::temp_dir().join("hbc2-cc-windows");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("main.c"), MAIN).unwrap();
    std::fs::write(dir.join("mark.has"), "_mark:\n    ret\n").unwrap();
    let library = build_library(&dir.join("library")).unwrap_or_else(|e| panic!("{}", report(&e)));
    let build = build_program(
        &[dir.join("main.c"), dir.join("mark.has")],
        &library,
        &dir.join("out"),
    )
    .unwrap_or_else(|e| panic!("{}", report(&e)));
    let mark = build.program.symbol("_mark").unwrap().address as u16;
    let mut m = Machine::new(MachineConfig::default(), Some(&build.program.image)).unwrap();
    m.breakpoints.set(mark, true);
    let first = m.run(10_000_000);
    assert_eq!(first.stop, Some(StopReason::Breakpoint(mark)));

    let (write, write_cycles) = measure(&mut m, mark);
    let (recolor, recolor_cycles) = measure(&mut m, mark);
    let (others, _) = measure(&mut m, mark);
    println!("screen_write, 768 characters: {write_cycles} cycles, longest off {write}");
    println!("screen_set_bg: {recolor_cycles} cycles, longest off {recolor}");
    println!("clear, line, screen_set_txt: longest off {others}");
    // One PUTS of 255 characters: about 52 cycles a byte.
    assert!(write < 14_500, "screen_write: {write} cycles off");
    assert!(write > 12_000, "one PUTS of 255 is the longest: {write}");
    // screen_set_bg: one RECOLOR, 3 bytes.
    assert!(recolor < 500, "screen_set_bg: {recolor} cycles off");
    assert!(recolor_cycles < 1_000, "screen_set_bg: {recolor_cycles} cycles");
    assert!(others < 1_000, "{others} cycles off");
}
