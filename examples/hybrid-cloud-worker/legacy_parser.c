#include <stdint.h>

// Compiled with the system C compiler during source preparation.
double parse_legacy_sensor(double raw_voltage, double calibration_factor) {
    // Simulate some esoteric, decades-old hardware parsing logic
    double baseline = raw_voltage * 3.14159;
    return (baseline / 100.0) + calibration_factor;
}
