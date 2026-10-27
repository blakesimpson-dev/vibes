//! `vibes run`: listen to the keyboard and play the pack. It all happens on the main thread's
//! run loop: the event tap's callback plays the sounds, and a timer follows the state file and
//! closes the audio output after a while without sound.

use crate::keys;
use crate::pack::Pack;
use crate::state::{self, State};
use core_foundation::base::TCFType;
use core_foundation::date::CFDate;
use core_foundation::runloop::{CFRunLoop, CFRunLoopTimer, CFRunLoopTimerRef, kCFRunLoopCommonModes};
use core_graphics::event::{
    CGEvent, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType, CallbackResult,
    EventField,
};
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Source};
use std::cell::RefCell;
use std::ffi::c_void;
use std::fs;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::time::{Duration, Instant, SystemTime};

/// How often the state file is checked.
const POLL_SECS: f64 = 0.5;
/// An open output keeps CoreAudio busy, so it's closed after this long without a sound. This
/// also moves to a new default output (headphones) on the next key.
const IDLE: Duration = Duration::from_secs(30);
const CAPS_LOCK: u16 = 0x39;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGPreflightListenEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;
    fn CGEventTapEnable(tap: *mut c_void, enable: bool);
}

/// The tap, so the callback can turn it back on when macOS turns it off.
static TAP: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

thread_local! {
    static ENGINE: RefCell<Engine> = RefCell::new(Engine::default());
}

#[derive(Default)]
struct Engine {
    state: State,
    /// The state file's modified time when it was last read; None before the first read.
    seen: Option<Option<SystemTime>>,
    pack: Option<Pack>,
    output: Option<MixerDeviceSink>,
    last_sound: Option<Instant>,
}

impl Engine {
    /// Reread the state if the file changed, and load the pack if that changed.
    fn refresh(&mut self) {
        let modified = fs::metadata(state::state_path()).and_then(|m| m.modified()).ok();
        if self.seen == Some(modified) {
            return;
        }
        let first = self.seen.is_none();
        self.seen = Some(modified);
        let new = State::load();
        if first || new.pack != self.state.pack {
            self.pack = if new.pack.is_empty() {
                eprintln!("vibes: no pack installed in {}", state::packs_dir().display());
                None
            } else {
                match Pack::load(&state::packs_dir().join(&new.pack)) {
                    Ok(pack) => {
                        eprintln!("vibes: playing {} ({})", new.pack, pack.name);
                        for warning in &pack.warnings {
                            eprintln!("vibes: skipped {warning}");
                        }
                        Some(pack)
                    }
                    Err(e) => {
                        eprintln!("vibes: can't load {}: {e}", new.pack);
                        None
                    }
                }
            };
        }
        self.state = new;
    }

    fn tick(&mut self) {
        self.refresh();
        if self.output.is_some() && self.last_sound.is_some_and(|t| t.elapsed() > IDLE) {
            self.output = None;
        }
    }

    fn key(&mut self, kind: CGEventType, event: &CGEvent) {
        if !self.state.enabled {
            return;
        }
        let mac = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u16;
        let pressed = match kind {
            CGEventType::KeyDown if event.get_integer_value_field(EventField::KEYBOARD_EVENT_AUTOREPEAT) != 0 => return,
            CGEventType::KeyDown => true,
            CGEventType::KeyUp => false,
            // Modifiers only send FlagsChanged, one per change of that key: its bit says which way.
            CGEventType::FlagsChanged => match keys::modifier_mask(mac) {
                Some(mask) => event.get_flags().bits() & mask != 0,
                None if mac == CAPS_LOCK => true, // one event per press, none for the release
                None => return,
            },
            _ => return,
        };
        let (Some(code), Some(pack)) = (keys::mechvibes_code(mac), &self.pack) else { return };
        let sound = if pressed { pack.press(code) } else { pack.release(code) };
        let Some(sound) = sound else { return };

        if self.output.is_none() {
            match DeviceSinkBuilder::open_default_sink() {
                Ok(mut output) => {
                    output.log_on_drop(false);
                    self.output = Some(output);
                }
                Err(e) => {
                    eprintln!("vibes: can't open the audio output: {e}");
                    return;
                }
            }
        }
        let volume = self.state.volume.min(100) as f32 / 100.0;
        if let Some(output) = &self.output {
            output.mixer().add(sound.amplify(volume));
        }
        self.last_sound = Some(Instant::now());
    }
}

extern "C" fn on_timer(_: CFRunLoopTimerRef, _: *mut c_void) {
    ENGINE.with_borrow_mut(Engine::tick);
}

pub fn run() -> Result<(), String> {
    if !unsafe { CGPreflightListenEventAccess() } {
        unsafe { CGRequestListenEventAccess() };
        return Err("needs Input Monitoring: turn it on for vibes in System Settings → Privacy & Security → \
                    Input Monitoring, then start it again"
            .into());
    }
    ENGINE.with_borrow_mut(Engine::refresh);

    let tap = CGEventTap::new(
        CGEventTapLocation::Session,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::ListenOnly,
        vec![CGEventType::KeyDown, CGEventType::KeyUp, CGEventType::FlagsChanged],
        |_proxy, kind, event| {
            match kind {
                // macOS turns a tap off if it's slow to answer (say, while a pack loads).
                CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput => unsafe {
                    CGEventTapEnable(TAP.load(Ordering::Relaxed), true)
                },
                _ => ENGINE.with_borrow_mut(|engine| engine.key(kind, event)),
            }
            CallbackResult::Keep
        },
    )
    .map_err(|()| "couldn't listen to the keyboard (is Input Monitoring on for vibes?)")?;
    TAP.store(tap.mach_port().as_concrete_TypeRef() as *mut c_void, Ordering::Relaxed);

    let run_loop = CFRunLoop::get_current();
    let source = tap.mach_port().create_runloop_source(0).map_err(|()| "couldn't add the keyboard listener")?;
    let timer =
        CFRunLoopTimer::new(CFDate::now().abs_time() + POLL_SECS, POLL_SECS, 0, 0, on_timer, std::ptr::null_mut());
    unsafe {
        run_loop.add_source(&source, kCFRunLoopCommonModes);
        run_loop.add_timer(&timer, kCFRunLoopCommonModes);
    }
    tap.enable();
    eprintln!("vibes: listening");
    CFRunLoop::run_current();
    Ok(())
}
