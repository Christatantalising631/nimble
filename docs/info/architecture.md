# NIMBLE RUNTIME ARCHITECTURE

## EXECUTION MODEL
Nimble uses a high-performance register-based virtual machine. Instructions are fixed 32-bit words for predictable fetch and dispatch.

### INSTRUCTION WORD
```text
[ opcode: 8 | dst: 8 | src1: 8 | src2: 8 ]
[ opcode: 8 | dst: 8 | imm16: 16         ]
```

## VALUE REPRESENTATION (NaN-BOXING)
All Nimble values are stored in a single 64-bit `u64`.
- **Floats:** Native IEEE 754 doubles.
- **Ints:** 32-bit payload with NaN tag.
- **Bools/Nil:** Specialized NaN tags.
- **Pointers:** 48-bit addresses to heap-allocated objects.

## OBJECT SYSTEM
Objects use **Shapes (Hidden Classes)** to eliminate repeated field lookups.
- Field access is a fixed-offset load from the object's slot array.
- Adding properties triggers a shape transition.

## JIT COMPILATION
Nimble uses a tiered JIT strategy:
1. **Tier 1 (Baseline):** Fast bytecode-to-machine-code translation using Cranelift.
2. **Tier 2 (Optimizing):** SSA-based optimization with type feedback and speculative inlining.

## MEMORY MANAGEMENT
- **Nursery (2MB):** Fast-path bump allocator for short-lived objects.
- **Old Generation:** Mark-Sweep collector for long-lived objects.
- **Safe Points:** Integrated into JIT and VM loops for deterministic collection.
