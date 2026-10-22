// Host-side microbenchmark reference for the STM32 hardware-signing station.
// This is the same code that runs on the STM32, but compiled for the host so
// we can verify the timing numbers before flashing the device.
//
// The STM32 signs a 104-byte message: 102-byte protobuf payload + 2 flags.
// This benchmark uses the SAME message and the SAME HMAC-SHA256 implementation.
//
// Table 4 format:
//   avg_latency_us : average round-trip signing latency in µs
//   pct_of_build   : percentage of the 14.8 ms end-to-end build time

use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::Instant;

type HmacSha256 = Hmac<Sha256>;

const CANON_PAYLOAD_LEN: usize = 102;
const FLAGS_LEN: usize = 2;
const TOTAL_SIGNED_LEN: usize = CANON_PAYLOAD_LEN + FLAGS_LEN; // 104
const ITERATIONS: u32 = 1000;
const BUILD_TIME_NS: u64 = 14_800_000; // 14.8 ms

fn hmac_sha256(key: &[u8], msg: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key");
    mac.update(msg);
    mac.finalize().into_bytes().to_vec()
}

fn build_signed_message(payload: &[u8], borrow: u8, typeck: u8) -> Vec<u8> {
    assert_eq!(payload.len(), CANON_PAYLOAD_LEN);
    let mut msg = Vec::with_capacity(TOTAL_SIGNED_LEN);
    msg.extend_from_slice(payload);
    msg.push(borrow);
    msg.push(typeck);
    msg
}

fn benchmark(key: &[u8], msg: &[u8]) -> (u64, Vec<u8>) {
    let mut total_ns: u64 = 0;
    let mut last_sig = Vec::new();
    for _ in 0..ITERATIONS {
        let start = Instant::now();
        last_sig = hmac_sha256(key, msg);
        let elapsed = start.elapsed().as_nanos() as u64;
        total_ns += elapsed;
    }
    let avg_ns = total_ns / (ITERATIONS as u64);
    let avg_us = avg_ns / 1_000;
    (avg_us, last_sig)
}

fn main() {
    // Use the canonical 102-byte buffer from /tmp/canonical_buffer.bin
    // (the protobuf payload), plus the two flags, to build the 104-byte
    // message that the STM32 signs.
    let payload_hex = std::fs::read_to_string("/tmp/canonical_buffer_hex.txt")
        .expect("missing /tmp/canonical_buffer_hex.txt");
    let payload = hex::decode(payload_hex.trim()).expect("invalid payload hex");
    assert_eq!(payload.len(), CANON_PAYLOAD_LEN);

    let key_hex = std::fs::read_to_string("/tmp/signing_key_hex.txt")
        .expect("missing /tmp/signing_key_hex.txt");
    let key = hex::decode(key_hex.trim()).expect("invalid key hex");

    // Build the 104-byte message with both flags set (the STM32 only signs
    // when the AND gate is asserted, which means both flags are true).
    let msg = build_signed_message(&payload, 1, 1);

    let (avg_us, sig) = benchmark(&key, &msg);
    let pct = (avg_us as f64 * 1_000.0 / BUILD_TIME_NS as f64) * 100.0;

    println!("=== STM32 + Hardware AND-Gate Micro-Benchmark (host reference) ===");
    println!("iterations      = {}", ITERATIONS);
    println!("signed_message  = {} bytes ({} protobuf + {} flags)", TOTAL_SIGNED_LEN, CANON_PAYLOAD_LEN, FLAGS_LEN);
    println!("payload_hex     = {}", payload_hex.trim());
    println!("key_hex         = {}", key_hex.trim());
    println!("avg_latency_us  = {} µs", avg_us);
    println!("avg_latency_ms  = {:.3} ms", avg_us as f64 / 1_000_0);
    println!("pct_of_build    = {:.2}% of 14.8 ms end-to-end build time", pct);
    println!("signature_hex   = {}", hex::encode(&sig));
    println!("=== END ===");

    // Write the results to a file for the paper
    let results = format!(
        "STM32 + Hardware AND-Gate Micro-Benchmark (host reference)\n\
         iterations      = {}\n\
         signed_message  = {} bytes ({} protobuf + {} flags)\n\
         payload_hex     = {}\n\
         key_hex         = {}\n\
         avg_latency_us  = {} µs\n\
         avg_latency_ms  = {:.3} ms\n\
         pct_of_build    = {:.2}% of 14.8 ms end-to-end build time\n\
         signature_hex   = {}\n",
        ITERATIONS,
        TOTAL_SIGNED_LEN,
        CANON_PAYLOAD_LEN,
        FLAGS_LEN,
        payload_hex.trim(),
        key_hex.trim(),
        avg_us,
        avg_us as f64 / 1_000_0,
        pct,
        hex::encode(&sig),
    );
    std::fs::write("/tmp/stm32_bench_results.txt", &results).expect("failed to write results");
    println!("Results written to /tmp/stm32_bench_results.txt");
}
