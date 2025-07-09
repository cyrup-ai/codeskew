// String processing support for WGSL
// Adapted from wgpu-compute-toy standard library

// String structure definition
struct String {
    length: u32,
    chars: array<u32, 20>, // STRING_MAX_LEN
}

// String creation helpers
fn createString(len: u32, chars: array<u32, 20>) -> String {
    return String(len, chars);
}

fn emptyString() -> String {
    return String(0u, array<u32, 20>());
}

// String manipulation
fn stringLength(s: String) -> u32 {
    return s.length;
}

fn stringChar(s: String, index: u32) -> u32 {
    if (index >= s.length) {
        return 0u;
    }
    return s.chars[index];
}

fn stringEquals(a: String, b: String) -> bool {
    if (a.length != b.length) {
        return false;
    }
    
    for (var i = 0u; i < a.length; i++) {
        if (a.chars[i] != b.chars[i]) {
            return false;
        }
    }
    
    return true;
}

// Character utilities
fn isDigit(c: u32) -> bool {
    return c >= 48u && c <= 57u; // '0' to '9'
}

fn isLetter(c: u32) -> bool {
    return (c >= 65u && c <= 90u) || (c >= 97u && c <= 122u); // 'A'-'Z' or 'a'-'z'
}

fn isAlphanumeric(c: u32) -> bool {
    return isDigit(c) || isLetter(c);
}

fn isWhitespace(c: u32) -> bool {
    return c == 32u || c == 9u || c == 10u || c == 13u; // space, tab, newline, carriage return
}

fn toLower(c: u32) -> u32 {
    if (c >= 65u && c <= 90u) { // 'A' to 'Z'
        return c + 32u;
    }
    return c;
}

fn toUpper(c: u32) -> u32 {
    if (c >= 97u && c <= 122u) { // 'a' to 'z'
        return c - 32u;
    }
    return c;
}

// String search
fn stringContains(haystack: String, needle: String) -> bool {
    if (needle.length > haystack.length) {
        return false;
    }
    
    for (var i = 0u; i <= haystack.length - needle.length; i++) {
        var found = true;
        for (var j = 0u; j < needle.length; j++) {
            if (haystack.chars[i + j] != needle.chars[j]) {
                found = false;
                break;
            }
        }
        if (found) {
            return true;
        }
    }
    
    return false;
}

fn stringStartsWith(s: String, prefix: String) -> bool {
    if (prefix.length > s.length) {
        return false;
    }
    
    for (var i = 0u; i < prefix.length; i++) {
        if (s.chars[i] != prefix.chars[i]) {
            return false;
        }
    }
    
    return true;
}

fn stringEndsWith(s: String, suffix: String) -> bool {
    if (suffix.length > s.length) {
        return false;
    }
    
    let offset = s.length - suffix.length;
    for (var i = 0u; i < suffix.length; i++) {
        if (s.chars[offset + i] != suffix.chars[i]) {
            return false;
        }
    }
    
    return true;
}

// Number to string conversion (basic)
fn digitToChar(digit: u32) -> u32 {
    return 48u + digit; // '0' + digit
}

fn charToDigit(c: u32) -> u32 {
    if (isDigit(c)) {
        return c - 48u;
    }
    return 0u;
}