import json

def testHttpGetThroughput(repeatLimit):
    checksum = 0
    repeatLimit = int(repeatLimit)
    for repeat in range(repeatLimit):
        req = "GET /api/data?id=" + str(repeat) + " HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n"
        res = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 15\r\n\r\n{\"status\":\"ok\"}"
        checksum += len(req) + len(res)
    return float(checksum)

smallJsonStr = json.dumps({
    "id": "1234567890", "name": "Performance Test Small", "description": "This is a small JSON payload for parsing and stringifying.", "tags": ["benchmark", "json", "parse", "performance", "small"], "metadata": { "createdAt": "2026-01-01T00:00:00Z", "active": True, "count": 42 }, "padding": "x" * 700
})

def testJsonParseSmall(repeatLimit):
    checksum = 0
    repeatLimit = int(repeatLimit)
    for repeat in range(repeatLimit):
        parsed = json.loads(smallJsonStr)
        checksum += len(parsed["padding"])
    return float(checksum)

largeJsonStr = json.dumps({
    "items": [{"id": i, "name": "Item " + str(i), "value": i * 1.5, "padding": "x" * 50} for i in range(1000)]
})

def testJsonParseLarge(repeatLimit):
    checksum = 0
    repeatLimit = int(repeatLimit)
    for repeat in range(repeatLimit):
        parsed = json.loads(largeJsonStr)
        checksum += parsed["items"][0]["id"]
    return float(checksum)

smallJsonObj = json.loads(smallJsonStr)

def testJsonStringifySmall(repeatLimit):
    checksum = 0
    repeatLimit = int(repeatLimit)
    for repeat in range(repeatLimit):
        checksum += len(json.dumps(smallJsonObj, separators=(',', ':')))
    return float(checksum)

mediumJsonObj = { "items": [{"id": i, "active": i % 2 == 0} for i in range(100)] }

def testJsonStringifyMedium(repeatLimit):
    checksum = 0
    repeatLimit = int(repeatLimit)
    for repeat in range(repeatLimit):
        checksum += len(json.dumps(mediumJsonObj, separators=(',', ':')))
    return float(checksum)

buf1kb = bytearray([i & 0xff for i in range(1024)])

def testSha256Small(repeatLimit):
    checksum = 0
    repeatLimit = int(repeatLimit)
    for repeat in range(repeatLimit):
        a, b, c, d = 0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a
        for offset in range(0, len(buf1kb), 64):
            w0 = buf1kb[offset] + (buf1kb[offset + 1] << 8)
            w1 = buf1kb[offset + 4] + (buf1kb[offset + 5] << 8)
            x0 = ((a + b) + w0 + 0x428a2f98) & 0xffffffff
            x1 = ((c + d) + w1 + 0x71374491) & 0xffffffff
            a, b = x0, x1
            checksum += (a + b) & 0xffffffff
    return float(checksum)

buf64kb = bytearray([i & 0xff for i in range(64 * 1024)])

def testSha256Large(repeatLimit):
    checksum = 0
    repeatLimit = int(repeatLimit)
    for repeat in range(repeatLimit):
        a, b, c, d = 0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a
        for offset in range(0, len(buf64kb), 64):
            w0 = buf64kb[offset] + (buf64kb[offset + 1] << 8)
            w1 = buf64kb[offset + 4] + (buf64kb[offset + 5] << 8)
            x0 = ((a + b) + w0 + 0x428a2f98) & 0xffffffff
            x1 = ((c + d) + w1 + 0x71374491) & 0xffffffff
            a, b = x0, x1
            checksum += (a + b) & 0xffffffff
    return float(checksum)

def testBufferCopy(repeatLimit):
    checksum = 0
    repeatLimit = int(repeatLimit)
    for repeat in range(repeatLimit):
        dst = bytearray(buf64kb)
        checksum += dst[0] + dst[-1]
    return float(checksum)

arrMapReduce = list(range(10000))

def testArrayMapReduce(repeatLimit):
    checksum = 0
    repeatLimit = int(repeatLimit)
    for repeat in range(repeatLimit):
        mapped = [x * 3 + 7 for x in arrMapReduce]
        checksum += sum(mapped)
    return float(checksum)

def testStringConcat(repeatLimit):
    checksum = 0
    repeatLimit = int(repeatLimit)
    for repeat in range(repeatLimit):
        s = "".join([str(i) for i in range(1000)])
        checksum += len(s)
    return float(checksum)

def testIntegerLoop(repeatLimit):
    checksum = 0
    repeatLimit = int(repeatLimit)
    for repeat in range(repeatLimit):
        value = 0
        for i in range(10000):
            value = (value + i * 3) % 997
        checksum += value
    return float(checksum)

randomInts = [((i * 137 + 7) % 1000) for i in range(10000)]

def testIntegerLoopRandom(repeatLimit):
    checksum = 0
    repeatLimit = int(repeatLimit)
    for repeat in range(repeatLimit):
        value = 0
        for i in range(10000):
            value = (value + randomInts[i]) % 997
        checksum += value
    return float(checksum)
