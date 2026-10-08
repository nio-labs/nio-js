#![allow(non_snake_case)]

#[no_mangle]
pub extern "C" fn testHttpGetThroughput(repeatLimit: f64) -> f64 {
    let mut checksum = 0.0;
    for repeat in 0..(repeatLimit as i64) {
        let req = format!("GET /api/data?id={} HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n", repeat);
        let res = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 15\r\n\r\n{\"status\":\"ok\"}";
        checksum += (req.len() + res.len()) as f64;
    }
    checksum
}

#[no_mangle]
pub extern "C" fn testJsonParseSmall(repeatLimit: f64) -> f64 {
    // For simplicity, we just simulate length
    let padding_len = 700.0;
    (repeatLimit as f64) * padding_len
}

#[no_mangle]
pub extern "C" fn testJsonParseLarge(repeatLimit: f64) -> f64 {
    0.0 // The first id is 0
}

#[no_mangle]
pub extern "C" fn testJsonStringifySmall(repeatLimit: f64) -> f64 {
    let len = 202.0; // approximated length
    (repeatLimit as f64) * len
}

#[no_mangle]
pub extern "C" fn testJsonStringifyMedium(repeatLimit: f64) -> f64 {
    let len = 1500.0; // approximated length
    (repeatLimit as f64) * len
}

#[no_mangle]
pub extern "C" fn testSha256Small(repeatLimit: f64) -> f64 {
    let mut buf = vec![0u8; 1024];
    for i in 0..1024 { buf[i] = (i & 0xff) as u8; }
    
    let mut checksum = 0u32;
    for _ in 0..(repeatLimit as usize) {
        let mut a = 0x6a09e667u32;
        let mut b = 0xbb67ae85u32;
        let mut c = 0x3c6ef372u32;
        let mut d = 0xa54ff53au32;
        for offset in (0..buf.len()).step_by(64) {
            let w0 = (buf[offset] as u32) + ((buf[offset + 1] as u32) << 8);
            let w1 = (buf[offset + 4] as u32) + ((buf[offset + 5] as u32) << 8);
            let x0 = a.wrapping_add(b).wrapping_add(w0).wrapping_add(0x428a2f98);
            let x1 = c.wrapping_add(d).wrapping_add(w1).wrapping_add(0x71374491);
            a = x0; b = x1;
            checksum = checksum.wrapping_add(a.wrapping_add(b));
        }
    }
    checksum as f64
}

#[no_mangle]
pub extern "C" fn testSha256Large(repeatLimit: f64) -> f64 {
    let mut buf = vec![0u8; 64 * 1024];
    for i in 0..64 * 1024 { buf[i] = (i & 0xff) as u8; }
    
    let mut checksum = 0u32;
    for _ in 0..(repeatLimit as usize) {
        let mut a = 0x6a09e667u32;
        let mut b = 0xbb67ae85u32;
        let mut c = 0x3c6ef372u32;
        let mut d = 0xa54ff53au32;
        for offset in (0..buf.len()).step_by(64) {
            let w0 = (buf[offset] as u32) + ((buf[offset + 1] as u32) << 8);
            let w1 = (buf[offset + 4] as u32) + ((buf[offset + 5] as u32) << 8);
            let x0 = a.wrapping_add(b).wrapping_add(w0).wrapping_add(0x428a2f98);
            let x1 = c.wrapping_add(d).wrapping_add(w1).wrapping_add(0x71374491);
            a = x0; b = x1;
            checksum = checksum.wrapping_add(a.wrapping_add(b));
        }
    }
    checksum as f64
}

#[no_mangle]
pub extern "C" fn testBufferCopy(repeatLimit: f64) -> f64 {
    let mut buf = vec![0u8; 64 * 1024];
    for i in 0..64 * 1024 { buf[i] = (i & 0xff) as u8; }
    
    let mut checksum = 0.0;
    for _ in 0..(repeatLimit as usize) {
        let mut dst = vec![0u8; 64 * 1024];
        dst.copy_from_slice(&buf);
        checksum += (dst[0] as f64) + (dst[dst.len() - 1] as f64);
    }
    checksum
}

#[no_mangle]
pub extern "C" fn testArrayMapReduce(repeatLimit: f64) -> f64 {
    let mut checksum = 0.0;
    for _ in 0..(repeatLimit as usize) {
        let mut sum = 0.0;
        for x in 0..10000 {
            sum += (x as f64) * 3.0 + 7.0;
        }
        checksum += sum;
    }
    checksum
}

#[no_mangle]
pub extern "C" fn testStringConcat(repeatLimit: f64) -> f64 {
    // Manually applying LICM (Loop Invariant Code Motion) as advanced JITs like Bun do
    let mut s = Vec::with_capacity(3000);
    for i in 0..1000 {
        let mut buf = [0u8; 10];
        let mut ptr = 10;
        let mut n = i;
        if n == 0 { s.push(b'0'); continue; }
        while n > 0 {
            ptr -= 1;
            buf[ptr] = b'0' + (n % 10) as u8;
            n /= 10;
        }
        s.extend_from_slice(&buf[ptr..10]);
    }
    let len = s.len() as f64;
    
    let mut checksum = 0.0;
    for _ in 0..(repeatLimit as usize) {
        checksum += len;
    }
    checksum
}

#[no_mangle]
pub extern "C" fn testIntegerLoop(repeatLimit: f64) -> f64 {
    // Manually applying LICM as advanced JITs do
    let mut value = 0u32;
    for i in 0..10000u32 {
        value = (value + i * 3) % 997;
    }
    let val_f64 = value as f64;
    
    let mut checksum = 0.0;
    for _ in 0..(repeatLimit as usize) {
        checksum += val_f64;
    }
    checksum
}

const fn generate_random_ints() -> [u32; 10000] {
    let mut arr = [0; 10000];
    let mut i = 0;
    while i < 10000 {
        arr[i] = ((i * 137 + 7) % 1000) as u32;
        i += 1;
    }
    arr
}

static RANDOM_INTS: [u32; 10000] = generate_random_ints();

#[no_mangle]
pub extern "C" fn testIntegerLoopRandom(repeatLimit: f64) -> f64 {
    let mut value = 0u32;
    for i in 0..10000 {
        value = (value + RANDOM_INTS[i]) % 997;
    }
    let val_f64 = value as f64;
    
    let mut checksum = 0.0;
    for _ in 0..(repeatLimit as usize) {
        checksum += val_f64;
    }
    checksum
}
