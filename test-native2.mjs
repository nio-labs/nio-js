/** @native */
function sha256_mock(data) {
  let checksum = 0;
  const h0 = 0x6a09e667;
  const h1 = 0xbb67ae85;
  const h2 = 0x3c6ef372;
  const h3 = 0xa54ff53a;
  let a = h0, b = h1, c = h2, d = h3;
  for (let offset = 0; offset < data.length; offset += 64) {
    const w0 = (data[offset] + (data[offset + 1] << 8)) | 0;
    const w1 = (data[offset + 4] + (data[offset + 5] << 8)) | 0;
    const x0 = ((a + b) + w0 + 0x428a2f98) | 0;
    const x1 = ((c + d) + w1 + 0x71374491) | 0;
    a = x0; b = x1;
    checksum += (a + b) >>> 0;
  }
  return checksum;
}
console.log(sha256_mock(new Uint8Array(64)));
