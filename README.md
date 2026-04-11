# cuda-axiom

Axiom language core — post-human programming language with JSON payloads, confidence types, payload tree execution

Part of the Cocapn fleet — a Lucineer vessel component.

## What It Does

### Key Types

- `Value` — core data structure
- `Constraint` — core data structure
- `CompiledOp` — core data structure
- `CompiledProgram` — core data structure
- `AxiomCompiler` — core data structure
- `AxiomVM` — core data structure
- _and 1 more (see source)_

## Quick Start

```bash
# Clone
git clone https://github.com/Lucineer/cuda-axiom.git
cd cuda-axiom

# Build
cargo build

# Run tests
cargo test
```

## Usage

```rust
use cuda_axiom::*;

// See src/lib.rs for full API
// 23 unit tests included
```

### Available Implementations

- `Value` — see source for methods
- `fmt::Display for Value` — see source for methods
- `AxiomType` — see source for methods
- `AxiomCompiler` — see source for methods
- `AxiomVM` — see source for methods

## Testing

```bash
cargo test
```

23 unit tests covering core functionality.

## Architecture

This crate is part of the **Cocapn Fleet** — a git-native multi-agent ecosystem.

- **Category**: other
- **Language**: Rust
- **Dependencies**: See `Cargo.toml`
- **Status**: Active development

## Related Crates


## Fleet Position

```
Casey (Captain)
├── JetsonClaw1 (Lucineer realm — hardware, low-level systems, fleet infrastructure)
├── Oracle1 (SuperInstance — lighthouse, architecture, consensus)
└── Babel (SuperInstance — multilingual scout)
```

## Contributing

This is a fleet vessel component. Fork it, improve it, push a bottle to `message-in-a-bottle/for-jetsonclaw1/`.

## License

MIT

---

*Built by JetsonClaw1 — part of the Cocapn fleet*
*See [cocapn-fleet-readme](https://github.com/Lucineer/cocapn-fleet-readme) for the full fleet roadmap*
