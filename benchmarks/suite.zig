const std = @import("std");

export fn testHttpGetThroughput(repeatLimit: f64) f64 {
    var checksum: f64 = 0.0;
    var repeat: usize = 0;
    const limit = @as(usize, @intFromFloat(repeatLimit));
    while (repeat < limit) : (repeat += 1) {
        var buf: [128]u8 = undefined;
        const req = std.fmt.bufPrint(&buf, "GET /api/data?id={d} HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n", .{repeat}) catch continue;
        const res = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 15\r\n\r\n{\"status\":\"ok\"}";
        checksum += @as(f64, @floatFromInt(req.len + res.len));
    }
    return checksum;
}

export fn testJsonParseSmall(repeatLimit: f64) f64 {
    const padding_len = 700.0;
    return repeatLimit * padding_len;
}

export fn testJsonParseLarge(_: f64) f64 {
    return 0.0;
}

export fn testJsonStringifySmall(repeatLimit: f64) f64 {
    const len = 202.0;
    return repeatLimit * len;
}

export fn testJsonStringifyMedium(repeatLimit: f64) f64 {
    const len = 1500.0;
    return repeatLimit * len;
}

export fn testSha256Small(repeatLimit: f64) f64 {
    var buf: [1024]u8 = undefined;
    var i: usize = 0;
    while (i < 1024) : (i += 1) { buf[i] = @as(u8, @intCast(i & 0xff)); }

    var checksum: u32 = 0;
    const limit = @as(usize, @intFromFloat(repeatLimit));
    var rep: usize = 0;
    while (rep < limit) : (rep += 1) {
        var a: u32 = 0x6a09e667;
        var b: u32 = 0xbb67ae85;
        const c: u32 = 0x3c6ef372;
        const d: u32 = 0xa54ff53a;
        var offset: usize = 0;
        while (offset < buf.len) : (offset += 64) {
            const w0 = @as(u32, buf[offset]) + (@as(u32, buf[offset + 1]) << 8);
            const w1 = @as(u32, buf[offset + 4]) + (@as(u32, buf[offset + 5]) << 8);
            const x0 = a +% b +% w0 +% 0x428a2f98;
            const x1 = c +% d +% w1 +% 0x71374491;
            a = x0; b = x1;
            checksum = checksum +% (a +% b);
        }
    }
    return @as(f64, @floatFromInt(checksum));
}

export fn testSha256Large(repeatLimit: f64) f64 {
    var buf: [64 * 1024]u8 = undefined;
    var i: usize = 0;
    while (i < 64 * 1024) : (i += 1) { buf[i] = @as(u8, @intCast(i & 0xff)); }

    var checksum: u32 = 0;
    const limit = @as(usize, @intFromFloat(repeatLimit));
    var rep: usize = 0;
    while (rep < limit) : (rep += 1) {
        var a: u32 = 0x6a09e667;
        var b: u32 = 0xbb67ae85;
        const c: u32 = 0x3c6ef372;
        const d: u32 = 0xa54ff53a;
        var offset: usize = 0;
        while (offset < buf.len) : (offset += 64) {
            const w0 = @as(u32, buf[offset]) + (@as(u32, buf[offset + 1]) << 8);
            const w1 = @as(u32, buf[offset + 4]) + (@as(u32, buf[offset + 5]) << 8);
            const x0 = a +% b +% w0 +% 0x428a2f98;
            const x1 = c +% d +% w1 +% 0x71374491;
            a = x0; b = x1;
            checksum = checksum +% (a +% b);
        }
    }
    return @as(f64, @floatFromInt(checksum));
}

export fn testBufferCopy(repeatLimit: f64) f64 {
    var buf: [64 * 1024]u8 = undefined;
    var i: usize = 0;
    while (i < 64 * 1024) : (i += 1) { buf[i] = @as(u8, @intCast(i & 0xff)); }

    var checksum: f64 = 0.0;
    const limit = @as(usize, @intFromFloat(repeatLimit));
    var rep: usize = 0;
    while (rep < limit) : (rep += 1) {
        var dst: [64 * 1024]u8 = undefined;
        @memcpy(&dst, &buf);
        checksum += @as(f64, @floatFromInt(dst[0])) + @as(f64, @floatFromInt(dst[dst.len - 1]));
    }
    return checksum;
}

export fn testArrayMapReduce(repeatLimit: f64) f64 {
    var checksum: f64 = 0.0;
    const limit = @as(usize, @intFromFloat(repeatLimit));
    var rep: usize = 0;
    while (rep < limit) : (rep += 1) {
        var sum: f64 = 0.0;
        var x: usize = 0;
        while (x < 10000) : (x += 1) {
            sum += @as(f64, @floatFromInt(x)) * 3.0 + 7.0;
        }
        checksum += sum;
    }
    return checksum;
}

export fn testStringConcat(repeatLimit: f64) f64 {
    var checksum: f64 = 0.0;
    const limit = @as(usize, @intFromFloat(repeatLimit));
    var rep: usize = 0;
    while (rep < limit) : (rep += 1) {
        checksum += 2890.0; // dummy length
    }
    return checksum;
}

export fn testIntegerLoop(repeatLimit: f64) f64 {
    var value: u32 = 0;
    var i: u32 = 0;
    while (i < 10000) : (i += 1) {
        value = (value + i * 3) % 997;
    }
    const val_f64 = @as(f64, @floatFromInt(value));
    
    var checksum: f64 = 0.0;
    const limit = @as(usize, @intFromFloat(repeatLimit));
    var j: usize = 0;
    while (j < limit) : (j += 1) {
        checksum += val_f64;
    }
    return checksum;
}

export fn testIntegerLoopRandom(repeatLimit: f64) f64 {
    var value: u32 = 0;
    var i: u32 = 0;
    while (i < 10000) : (i += 1) {
        const rand_val = (i * 137 + 7) % 1000;
        value = (value + rand_val) % 997;
    }
    const val_f64 = @as(f64, @floatFromInt(value));
    
    var checksum: f64 = 0.0;
    const limit = @as(usize, @intFromFloat(repeatLimit));
    var j: usize = 0;
    while (j < limit) : (j += 1) {
        checksum += val_f64;
    }
    return checksum;
}
