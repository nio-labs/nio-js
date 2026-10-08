import { get, reply } from 'nio.js';

// 1. Go Cloud-Native Bridge (via cgo shared library)
import { FetchClusterStatus } from './network.go';

// 2. C (Compiled during preparation)
import { parse_legacy_sensor } from './legacy_parser.c';

get('/api/iot/cluster-metrics', () => {
    try {
        // Fetch cluster networking data from Go
        const clusterDataStr = FetchClusterStatus();
        const cluster = JSON.parse(clusterDataStr);

        // Parse legacy sensor hardware data via Raw C
        const rawVoltage = 420.5;
        const calibration = 1.05;
        const sensorValue = parse_legacy_sensor(rawVoltage, calibration);

        return {
            source: 'hybrid-cloud-worker',
            timestamp: Date.now(),
            infrastructure: cluster,
            hardware_telemetry: {
                raw_voltage: rawVoltage,
                normalized_value: Number(sensorValue.toFixed(4))
            }
        };
    } catch (error) {
        return reply({ error: 'Failed to aggregate hybrid telemetry' }, { status: 500 });
    }
});
