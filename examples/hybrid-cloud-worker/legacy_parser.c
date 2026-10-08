#include <stdint.h>

// This raw C code is dynamically compiled in-memory via TinyCC (tcc)
// zero toolchain overhead, instant FFI binding to QuickJS.
double parse_legacy_sensor(double raw_voltage, double calibration_factor) {
    // Simulate some esoteric, decades-old hardware parsing logic
    double baseline = raw_voltage * 3.14159;
    return (baseline / 100.0) + calibration_factor;
}
