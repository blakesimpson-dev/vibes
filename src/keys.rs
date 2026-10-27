//! macOS key codes (kVK_*, from HIToolbox's Events.h) to the key codes Mechvibes packs use
//! (libuiohook's, which Mechvibes calls "standard").

/// Every key a Mechvibes v2 pack gives a sound, either its own or the pack's fallback.
pub const STANDARD: &[u32] = &[
    1, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 87, 88, 91, 92, 93, 41, 2, 3, 4, 5, 6, 7, 8, 9, 10,
    11, 12, 13, 14, 15, 58, 30, 48, 46, 32, 18, 33, 34, 35, 23, 36, 37, 38, 50, 49, 24, 25, 16, 19,
    31, 20, 22, 47, 17, 45, 21, 44, 26, 27, 43, 39, 40, 28, 51, 52, 53, 57, 3639, 70, 3653, 3666,
    3667, 3655, 3663, 3657, 3665, 57416, 57419, 57421, 57424, 42, 54, 29, 3613, 56, 3640, 3675,
    3676, 3677, 69, 3637, 55, 74, 3597, 78, 3612, 83, 79, 80, 81, 75, 76, 77, 71, 72, 73, 82,
];

/// The Mechvibes code for a macOS key code, or None for keys packs don't cover.
pub fn mechvibes_code(mac: u16) -> Option<u32> {
    Some(match mac {
        0x00 => 30,   // A
        0x01 => 31,   // S
        0x02 => 32,   // D
        0x03 => 33,   // F
        0x04 => 35,   // H
        0x05 => 34,   // G
        0x06 => 44,   // Z
        0x07 => 45,   // X
        0x08 => 46,   // C
        0x09 => 47,   // V
        0x0A => 41,   // § (ISO), where ` is on ANSI
        0x0B => 48,   // B
        0x0C => 16,   // Q
        0x0D => 17,   // W
        0x0E => 18,   // E
        0x0F => 19,   // R
        0x10 => 21,   // Y
        0x11 => 20,   // T
        0x12 => 2,    // 1
        0x13 => 3,    // 2
        0x14 => 4,    // 3
        0x15 => 5,    // 4
        0x16 => 7,    // 6
        0x17 => 6,    // 5
        0x18 => 13,   // =
        0x19 => 10,   // 9
        0x1A => 8,    // 7
        0x1B => 12,   // -
        0x1C => 9,    // 8
        0x1D => 11,   // 0
        0x1E => 27,   // ]
        0x1F => 24,   // O
        0x20 => 22,   // U
        0x21 => 26,   // [
        0x22 => 23,   // I
        0x23 => 25,   // P
        0x24 => 28,   // Return
        0x25 => 38,   // L
        0x26 => 36,   // J
        0x27 => 40,   // '
        0x28 => 37,   // K
        0x29 => 39,   // ;
        0x2A => 43,   // \
        0x2B => 51,   // ,
        0x2C => 53,   // /
        0x2D => 49,   // N
        0x2E => 50,   // M
        0x2F => 52,   // .
        0x30 => 15,   // Tab
        0x31 => 57,   // Space
        0x32 => 41,   // `
        0x33 => 14,   // Delete (backspace)
        0x35 => 1,    // Escape
        0x36 => 3676, // Right Command
        0x37 => 3675, // Command
        0x38 => 42,   // Shift
        0x39 => 58,   // Caps Lock
        0x3A => 56,   // Option
        0x3B => 29,   // Control
        0x3C => 54,   // Right Shift
        0x3D => 3640, // Right Option
        0x3E => 3613, // Right Control
        0x3F => 3666, // Fn (Mechvibes puts it where Insert is)
        0x41 => 83,   // Keypad .
        0x43 => 55,   // Keypad *
        0x45 => 78,   // Keypad +
        0x47 => 69,   // Keypad Clear
        0x4B => 3637, // Keypad /
        0x4C => 3612, // Keypad Enter
        0x4E => 74,   // Keypad -
        0x51 => 3597, // Keypad =
        0x52 => 82,   // Keypad 0
        0x53 => 79,   // Keypad 1
        0x54 => 80,   // Keypad 2
        0x55 => 81,   // Keypad 3
        0x56 => 75,   // Keypad 4
        0x57 => 76,   // Keypad 5
        0x58 => 77,   // Keypad 6
        0x59 => 71,   // Keypad 7
        0x5B => 72,   // Keypad 8
        0x5C => 73,   // Keypad 9
        0x60 => 63,   // F5
        0x61 => 64,   // F6
        0x62 => 65,   // F7
        0x63 => 61,   // F3
        0x64 => 66,   // F8
        0x65 => 67,   // F9
        0x67 => 87,   // F11
        0x69 => 91,   // F13
        0x6B => 92,   // F14
        0x6D => 68,   // F10
        0x6F => 88,   // F12
        0x71 => 93,   // F15
        0x72 => 3666, // Help (Insert)
        0x73 => 3655, // Home
        0x74 => 3657, // Page Up
        0x75 => 3667, // Forward Delete
        0x76 => 62,   // F4
        0x77 => 3663, // End
        0x78 => 60,   // F2
        0x79 => 3665, // Page Down
        0x7A => 59,   // F1
        0x7B => 57419, // Left
        0x7C => 57421, // Right
        0x7D => 57424, // Down
        0x7E => 57416, // Up
        _ => return None,
    })
}

/// For a modifier key, the bit in an event's flags that is set while that key is held
/// (the device-dependent bits, so left and right are told apart). None for other keys.
/// Caps Lock is left out: its flag is the lock, not the key.
pub fn modifier_mask(mac: u16) -> Option<u64> {
    Some(match mac {
        0x3B => 0x0000_0001, // Control
        0x38 => 0x0000_0002, // Shift
        0x3C => 0x0000_0004, // Right Shift
        0x37 => 0x0000_0008, // Command
        0x36 => 0x0000_0010, // Right Command
        0x3A => 0x0000_0020, // Option
        0x3D => 0x0000_0040, // Right Option
        0x3E => 0x0000_2000, // Right Control
        0x3F => 0x0080_0000, // Fn
        _ => return None,
    })
}
