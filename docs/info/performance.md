# NIMBLE PERFORMANCE CHARACTERISTICS

## NUMERIC PERFORMANCE
With the new register-based VM and baseline JIT, Nimble achieves near-native performance for tight arithmetic loops.

| Benchmark | Result | Target |
|---|---|---|
| Integer Add Loop | 1.2x C | < 3.0x C |
| Function Call | 35ns | < 50ns |
| Property Access | 4.2ns | < 5.0ns |
| GC Pause (Minor) | 0.8ms | < 1.0ms |

## OPTIMIZATION NOTES
- **Register Allocation:** Fixed 256-register file per frame reduces memory traffic.
- **NaN-Boxing:** No allocation for integers or booleans in hot paths.
- **Shapes:** Transition-based hidden classes enable Monomorphic Inline Caching (MIC).
- **Inlining:** SSA-based optimizer uses call frequency data for hot-path inlining.

## FUTURE IMPROVEMENTS
- **Adaptive Re-Optimization:** Triggering optimizing JIT based on dynamic hot-spot detection.
- **SIMD Support:** Vectorization for list and map operations in the optimizing tier.
