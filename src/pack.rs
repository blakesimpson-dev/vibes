//! A Mechvibes sound pack (a folder with a config.json), decoded into memory, read the way
//! Mechvibes reads it (src/libs/soundpacks/config-v1.js and config-v2.js in its repo):
//!
//! - `key_define_type` "single": `sound` is one file, and each `defines` entry is a
//!   [start ms, length ms] slice of it.
//! - "multi": each `defines` entry is a file of its own.
//! - Version 1 (or no version): keys without an entry are silent, and there are no release
//!   sounds.
//! - Version 2: release sounds are `"<code>-up"` entries. In multi packs, keys without an
//!   entry fall back to `sound` (press) and `soundup` (release). A `{0-4}` in a file name is
//!   a number picked once per key when the pack loads, so each key keeps its own sound, like
//!   switches on a real board.
//!
//! In multi packs a key whose file is missing or won't decode gets the fallback sound instead, or
//! is silent in a version 1 pack (see `warnings`); Mechvibes would refuse the whole pack.

use crate::keys::STANDARD;
use rodio::buffer::SamplesBuffer;
use rodio::{Decoder, Source};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Pack {
    pub name: String,
    /// Files that couldn't be used, so their keys are silent.
    pub warnings: Vec<String>,
    press: HashMap<u32, SamplesBuffer>,
    release: HashMap<u32, SamplesBuffer>,
}

impl Pack {
    pub fn press(&self, code: u32) -> Option<SamplesBuffer> {
        self.press.get(&code).cloned()
    }

    pub fn release(&self, code: u32) -> Option<SamplesBuffer> {
        self.release.get(&code).cloned()
    }

    /// How many keys have a press sound, and how many a release sound.
    pub fn counts(&self) -> (usize, usize) {
        (self.press.len(), self.release.len())
    }

    pub fn load(dir: &Path) -> Result<Pack, String> {
        let config_path = dir.join("config.json");
        let text = fs::read_to_string(&config_path).map_err(|e| format!("{}: {e}", config_path.display()))?;
        let config: Map<String, Value> =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", config_path.display()))?;
        let field = |name: &str| config.get(name).and_then(Value::as_str);

        let name = field("name").unwrap_or_default().to_string();
        let version = config.get("version").and_then(Value::as_u64).unwrap_or(1);
        let empty = Map::new();
        let defines = config.get("defines").and_then(Value::as_object).unwrap_or(&empty);
        let mut files = Files { dir, decoded: HashMap::new(), rng: Rng::new() };
        let mut pack = Pack { name, warnings: Vec::new(), press: HashMap::new(), release: HashMap::new() };

        match field("key_define_type") {
            Some("single") => {
                let sound = field("sound").ok_or("a single pack needs a \"sound\" file")?;
                let audio = files.get(sound)?;
                for (key, slice) in defines {
                    let Some([start, length]) = slice.as_array().and_then(|a| a.get(0..2)) else { continue };
                    let (Some(start), Some(length)) = (start.as_f64(), length.as_f64()) else { continue };
                    let (code, sounds) = match key.strip_suffix("-up") {
                        Some(code) if version >= 2 => (code, &mut pack.release),
                        Some(_) => continue,
                        None => (key.as_str(), &mut pack.press),
                    };
                    if let Ok(code) = code.parse() {
                        sounds.insert(code, audio.slice(start, length));
                    }
                }
            }
            Some("multi") => {
                let mut codes: Vec<u32> = defines.keys().filter_map(|k| k.trim_end_matches("-up").parse().ok()).collect();
                if version >= 2 {
                    codes.extend(STANDARD);
                }
                codes.sort();
                codes.dedup();
                // The files to try for a key, in order: its own, then (version 2) the fallback.
                let files_for = |key: String, fallback: Option<&str>| {
                    let own = defines.get(&key).and_then(Value::as_str).filter(|f| !f.is_empty());
                    own.into_iter().chain(fallback.filter(|_| version >= 2)).map(str::to_string).collect::<Vec<_>>()
                };
                for code in codes {
                    let press = files_for(code.to_string(), field("sound"));
                    let release = if version >= 2 { files_for(format!("{code}-up"), field("soundup")) } else { vec![] };
                    for (candidates, sounds) in [(press, &mut pack.press), (release, &mut pack.release)] {
                        for file in candidates {
                            match files.get(&file) {
                                Ok(audio) => {
                                    sounds.insert(code, audio.whole());
                                    break;
                                }
                                Err(e) if !pack.warnings.contains(&e) => pack.warnings.push(e),
                                Err(_) => {}
                            }
                        }
                    }
                }
            }
            other => return Err(format!("unknown key_define_type {other:?} (expected \"single\" or \"multi\")")),
        }

        if pack.press.is_empty() {
            return Err("no key sounds defined".into());
        }
        Ok(pack)
    }
}

/// A decoded audio file.
#[derive(Clone)]
struct Audio(SamplesBuffer);

impl Audio {
    fn whole(&self) -> SamplesBuffer {
        self.0.clone()
    }

    /// The part from start ms for length ms.
    fn slice(&self, start_ms: f64, length_ms: f64) -> SamplesBuffer {
        let (channels, rate) = (self.0.channels(), self.0.sample_rate());
        let frame = |ms: f64| (ms.max(0.0) / 1000.0 * rate.get() as f64) as usize * channels.get() as usize;
        let samples: Vec<_> = self.0.clone().skip(frame(start_ms)).take(frame(length_ms)).collect();
        SamplesBuffer::new(channels, rate, samples)
    }
}

/// The pack's files, each decoded once however many keys use it.
struct Files<'a> {
    dir: &'a Path,
    /// By file name (after `pick`), including the ones that failed, so each is tried once.
    decoded: HashMap<String, Result<Audio, String>>,
    rng: Rng,
}

impl Files<'_> {
    fn get(&mut self, name: &str) -> Result<Audio, String> {
        let name = self.pick(name);
        if let Some(audio) = self.decoded.get(&name) {
            return audio.clone();
        }
        let audio = self.decode(&name);
        self.decoded.insert(name, audio.clone());
        audio
    }

    fn decode(&self, name: &str) -> Result<Audio, String> {
        let path = self.dir.join(name);
        let err = |e: &dyn std::fmt::Display| format!("{}: {e}", path.display());
        let file = File::open(&path).map_err(|e| err(&e))?;
        let len = file.metadata().map_err(|e| err(&e))?.len();
        let mut builder = Decoder::builder().with_data(BufReader::new(file)).with_byte_len(len).with_seekable(true);
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            builder = builder.with_hint(ext);
        }
        let decoder = builder.build().map_err(|e| err(&e))?;
        let (channels, rate) = (decoder.channels(), decoder.sample_rate());
        Ok(Audio(SamplesBuffer::new(channels, rate, decoder.collect::<Vec<_>>())))
    }

    /// The file name with any {a-b} replaced by a number from a to b.
    fn pick(&mut self, name: &str) -> String {
        let (Some(open), Some(close)) = (name.find('{'), name.find('}')) else { return name.to_string() };
        let Some((a, b)) = name[open + 1..close].split_once('-') else { return name.to_string() };
        let (Ok(a), Ok(b)) = (a.trim().parse::<u64>(), b.trim().parse::<u64>()) else { return name.to_string() };
        let n = a + self.rng.next() % (b.max(a) - a + 1);
        format!("{}{n}{}", &name[..open], &name[close + 1..])
    }
}

/// xorshift64: plenty for picking sound variants.
struct Rng(u64);

impl Rng {
    fn new() -> Rng {
        let seed = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
        Rng(seed | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}
