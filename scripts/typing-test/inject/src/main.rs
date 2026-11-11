//! Live probe helper: send keys and mouse to the nested KWin through org_kde_kwin_fake_input.
//! Usage: inject click X Y | move X Y | type TEXT | key CODE [CODE..] | wait MS
use std::{thread::sleep, time::Duration};
use wayland_client::{
    Connection, Dispatch, QueueHandle,
    globals::{GlobalListContents, registry_queue_init},
    protocol::wl_registry,
};
use wayland_protocols_plasma::fake_input::client::org_kde_kwin_fake_input::OrgKdeKwinFakeInput;

struct S;
impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for S {
    fn event(_: &mut Self, _: &wl_registry::WlRegistry, _: wl_registry::Event, _: &GlobalListContents, _: &Connection, _: &QueueHandle<Self>) {}
}
impl Dispatch<OrgKdeKwinFakeInput, ()> for S {
    fn event(_: &mut Self, _: &OrgKdeKwinFakeInput, _: <OrgKdeKwinFakeInput as wayland_client::Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

/// What typing a character takes: character, evdev key code, Shift, AltGr, and whether the key is a dead key (the
/// character appears only after a Space).
type Key = (char, u32, bool, bool, bool);

// The strokes of the layouts, as xkbcomp compiles them (checked against it by tests/typing-test/keymap.sh, which
// compares `inject table <layout>` with scripts/typing-test/keymap.py). Letters and digits are in the tables too.
const DE: &[Key] = &[
    ('.', 52, false, false, false), (',', 51, false, false, false), ('-', 53, false, false, false),
    ('_', 53, true, false, false), ('+', 27, false, false, false), ('@', 16, false, true, false),
    ('/', 8, true, false, false), (':', 52, true, false, false), (';', 51, true, false, false),
    ('\'', 43, true, false, false), ('\"', 3, true, false, false), ('(', 9, true, false, false),
    (')', 10, true, false, false), ('[', 9, false, true, false), (']', 10, false, true, false),
    ('{', 8, false, true, false), ('}', 11, false, true, false), ('~', 27, false, true, false),
    ('^', 41, false, false, true), ('\\', 12, false, true, false), ('|', 86, false, true, false),
    ('€', 18, false, true, false), ('ä', 40, false, false, false), ('ö', 39, false, false, false),
    ('ü', 26, false, false, false), ('ß', 12, false, false, false), ('Ä', 40, true, false, false),
    ('Ö', 39, true, false, false), ('Ü', 26, true, false, false), ('!', 2, true, false, false),
    ('#', 43, false, false, false), ('$', 5, true, false, false), ('%', 6, true, false, false),
    ('&', 7, true, false, false), ('*', 27, true, false, false), ('<', 86, false, false, false),
    ('=', 11, true, false, false), ('>', 86, true, false, false), ('?', 12, true, false, false),
    ('µ', 50, false, true, false), ('§', 4, true, false, false), ('°', 41, true, false, false),
    ('a', 30, false, false, false), ('b', 48, false, false, false), ('c', 46, false, false, false),
    ('d', 32, false, false, false), ('e', 18, false, false, false), ('f', 33, false, false, false),
    ('g', 34, false, false, false), ('h', 35, false, false, false), ('i', 23, false, false, false),
    ('j', 36, false, false, false), ('k', 37, false, false, false), ('l', 38, false, false, false),
    ('m', 50, false, false, false), ('n', 49, false, false, false), ('o', 24, false, false, false),
    ('p', 25, false, false, false), ('q', 16, false, false, false), ('r', 19, false, false, false),
    ('s', 31, false, false, false), ('t', 20, false, false, false), ('u', 22, false, false, false),
    ('v', 47, false, false, false), ('w', 17, false, false, false), ('x', 45, false, false, false),
    ('y', 44, false, false, false), ('z', 21, false, false, false), ('0', 11, false, false, false),
    ('1', 2, false, false, false), ('2', 3, false, false, false), ('3', 4, false, false, false),
    ('4', 5, false, false, false), ('5', 6, false, false, false), ('6', 7, false, false, false),
    ('7', 8, false, false, false), ('8', 9, false, false, false), ('9', 10, false, false, false),
    ('A', 30, true, false, false), ('B', 48, true, false, false), ('C', 46, true, false, false),
    ('D', 32, true, false, false), ('E', 18, true, false, false), ('F', 33, true, false, false),
    ('G', 34, true, false, false), ('H', 35, true, false, false), ('I', 23, true, false, false),
    ('J', 36, true, false, false), ('K', 37, true, false, false), ('L', 38, true, false, false),
    ('M', 50, true, false, false), ('N', 49, true, false, false), ('O', 24, true, false, false),
    ('P', 25, true, false, false), ('Q', 16, true, false, false), ('R', 19, true, false, false),
    ('S', 31, true, false, false), ('T', 20, true, false, false), ('U', 22, true, false, false),
    ('V', 47, true, false, false), ('W', 17, true, false, false), ('X', 45, true, false, false),
    ('Y', 44, true, false, false), ('Z', 21, true, false, false), (' ', 57, false, false, false),
];

const US: &[Key] = &[
    ('.', 52, false, false, false), (',', 51, false, false, false), ('-', 12, false, false, false),
    ('_', 12, true, false, false), ('+', 13, true, false, false), ('@', 3, true, false, false),
    ('/', 53, false, false, false), (':', 39, true, false, false), (';', 39, false, false, false),
    ('\'', 40, false, false, false), ('\"', 40, true, false, false), ('(', 10, true, false, false),
    (')', 11, true, false, false), ('[', 26, false, false, false), (']', 27, false, false, false),
    ('{', 26, true, false, false), ('}', 27, true, false, false), ('~', 41, true, false, false),
    ('^', 7, true, false, false), ('\\', 43, false, false, false), ('|', 43, true, false, false),
    ('!', 2, true, false, false), ('#', 4, true, false, false), ('$', 5, true, false, false),
    ('%', 6, true, false, false), ('&', 8, true, false, false), ('*', 9, true, false, false),
    ('<', 86, false, false, false), ('=', 13, false, false, false), ('>', 52, true, false, false),
    ('?', 53, true, false, false), ('`', 41, false, false, false), ('a', 30, false, false, false),
    ('b', 48, false, false, false), ('c', 46, false, false, false), ('d', 32, false, false, false),
    ('e', 18, false, false, false), ('f', 33, false, false, false), ('g', 34, false, false, false),
    ('h', 35, false, false, false), ('i', 23, false, false, false), ('j', 36, false, false, false),
    ('k', 37, false, false, false), ('l', 38, false, false, false), ('m', 50, false, false, false),
    ('n', 49, false, false, false), ('o', 24, false, false, false), ('p', 25, false, false, false),
    ('q', 16, false, false, false), ('r', 19, false, false, false), ('s', 31, false, false, false),
    ('t', 20, false, false, false), ('u', 22, false, false, false), ('v', 47, false, false, false),
    ('w', 17, false, false, false), ('x', 45, false, false, false), ('y', 21, false, false, false),
    ('z', 44, false, false, false), ('0', 11, false, false, false), ('1', 2, false, false, false),
    ('2', 3, false, false, false), ('3', 4, false, false, false), ('4', 5, false, false, false),
    ('5', 6, false, false, false), ('6', 7, false, false, false), ('7', 8, false, false, false),
    ('8', 9, false, false, false), ('9', 10, false, false, false), ('A', 30, true, false, false),
    ('B', 48, true, false, false), ('C', 46, true, false, false), ('D', 32, true, false, false),
    ('E', 18, true, false, false), ('F', 33, true, false, false), ('G', 34, true, false, false),
    ('H', 35, true, false, false), ('I', 23, true, false, false), ('J', 36, true, false, false),
    ('K', 37, true, false, false), ('L', 38, true, false, false), ('M', 50, true, false, false),
    ('N', 49, true, false, false), ('O', 24, true, false, false), ('P', 25, true, false, false),
    ('Q', 16, true, false, false), ('R', 19, true, false, false), ('S', 31, true, false, false),
    ('T', 20, true, false, false), ('U', 22, true, false, false), ('V', 47, true, false, false),
    ('W', 17, true, false, false), ('X', 45, true, false, false), ('Y', 21, true, false, false),
    ('Z', 44, true, false, false), (' ', 57, false, false, false),
];

/// The table of `layout`; a layout without one is reported at once and ends the program (exit 2: the test did not run).
fn table(layout: &str) -> &'static [Key] {
    match layout {
        "de" => DE,
        "us" => US,
        other => {
            eprintln!("fs-typing-inject: no key table for layout '{other}' (there are: de, us)");
            std::process::exit(2);
        }
    }
}

/// The strokes that produce `c` on `layout`: key code, Shift, AltGr, dead.
fn stroke(layout: &str, c: char) -> Option<(u32, bool, bool, bool)> {
    table(layout).iter().find(|k| k.0 == c).map(|k| (k.1, k.2, k.3, k.4))
}

fn layout() -> String {
    std::env::var("INJECT_LAYOUT").unwrap_or_else(|_| "de".into())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("table") {
        // `inject table <layout>`: one line "char key shift altgr dead" per character, to compare with xkbcomp.
        for (c, k, sh, alt, dead) in table(args.get(1).map_or("de", String::as_str)) {
            println!("{c} {k} {} {} {}", u8::from(*sh), u8::from(*alt), u8::from(*dead));
        }
        return;
    }
    let conn = Connection::connect_to_env().expect("connect");
    let (globals, mut queue) = registry_queue_init::<S>(&conn).expect("registry");
    let qh = queue.handle();
    let fake: OrgKdeKwinFakeInput = globals.bind(&qh, 4..=5, ()).expect("fake_input missing");
    fake.authenticate("probe".into(), "live probe".into());
    let mut s = S;
    queue.roundtrip(&mut s).ok();
    if args.first().map(String::as_str) == Some("repl") {
        let stdin = std::io::stdin();
        for line in std::io::BufRead::lines(stdin.lock()) {
            let line = line.unwrap();
            let words: Vec<String> = line.split_whitespace().map(String::from).collect();
            if words.is_empty() { continue; }
            if words[0] == "quit" { return; }
            if words[0] == "text" {
                // Everything after `text ` is typed with the German layout, one stroke at a time.
                let rest = line.trim_start().strip_prefix("text").unwrap().strip_prefix(' ').unwrap_or("");
                text_typed(rest, &fake, &mut queue, &mut s);
                continue;
            }
            run(&words, &fake, &mut queue, &mut s);
        }
        return;
    }
    run(&args, &fake, &mut queue, &mut s);
}

fn run(args: &[String], fake: &OrgKdeKwinFakeInput, queue: &mut wayland_client::EventQueue<S>, s: &mut S) {
    let flush = |q: &mut wayland_client::EventQueue<S>, s: &mut S| { q.roundtrip(s).ok(); };
    let mut it = args.iter();
    while let Some(cmd) = it.next() {
        match cmd.as_str() {
            "move" | "click" => {
                let x: f64 = it.next().unwrap().parse().unwrap();
                let y: f64 = it.next().unwrap().parse().unwrap();
                fake.pointer_motion_absolute(x, y);
                flush(queue, s);
                if cmd == "click" {
                    sleep(Duration::from_millis(80));
                    fake.button(0x110, 1); flush(queue, s);
                    sleep(Duration::from_millis(60));
                    fake.button(0x110, 0); flush(queue, s);
                }
            }
            "type" => {
                for c in it.next().unwrap().chars() {
                    let Some((k, shift, _, _)) = stroke("us", c) else { continue };
                    if shift { fake.keyboard_key(42, 1); }
                    fake.keyboard_key(k, 1); flush(queue, s);
                    fake.keyboard_key(k, 0);
                    if shift { fake.keyboard_key(42, 0); }
                    flush(queue, s);
                    sleep(Duration::from_millis(40));
                }
            }
            "key" => {
                let k: u32 = it.next().unwrap().parse().unwrap();
                fake.keyboard_key(k, 1); flush(queue, s);
                fake.keyboard_key(k, 0); flush(queue, s);
            }
            "down" | "up" => {
                let k: u32 = it.next().unwrap().parse().unwrap();
                fake.keyboard_key(k, u32::from(cmd == "down"));
                flush(queue, s);
            }
            "wait" => sleep(Duration::from_millis(it.next().unwrap().parse().unwrap())),
            other => panic!("unknown {other}"),
        }
        sleep(Duration::from_millis(30));
    }
}

fn now_us() -> u64 {
    let mut t = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    // SAFETY: plain libc call with a valid pointer.
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut t) };
    t.tv_sec as u64 * 1_000_000 + t.tv_nsec as u64 / 1000
}

/// Type `text` on the layout of INJECT_LAYOUT (default de). Prints `KEY <char> <monotonic µs>` as each key goes down.
/// A dead key (the ^ of the German layout) is followed by a Space, which makes it type itself.
fn text_typed(text: &str, fake: &OrgKdeKwinFakeInput, queue: &mut wayland_client::EventQueue<S>, s: &mut S) {
    let pause: u64 = std::env::var("INJECT_PAUSE_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(120);
    let layout = layout();
    for c in text.chars() {
        let Some((k, shift, alt, dead)) = stroke(&layout, c) else { eprintln!("NOKEY {c}"); continue };
        let mut strokes = vec![k];
        if dead { strokes.push(57); }
        for (n, k) in strokes.into_iter().enumerate() {
            let (shift, alt) = if n == 0 { (shift, alt) } else { (false, false) };
            if shift { fake.keyboard_key(42, 1); }
            if alt { fake.keyboard_key(100, 1); }
            queue.roundtrip(s).ok();
            sleep(Duration::from_millis(15));
            if n == 0 { println!("KEY {c} {}", now_us()); }
            fake.keyboard_key(k, 1); queue.roundtrip(s).ok();
            sleep(Duration::from_millis(10));
            fake.keyboard_key(k, 0);
            if alt { fake.keyboard_key(100, 0); }
            if shift { fake.keyboard_key(42, 0); }
            queue.roundtrip(s).ok();
            sleep(Duration::from_millis(if n == 0 { 30 } else { pause }));
        }
    }
}
