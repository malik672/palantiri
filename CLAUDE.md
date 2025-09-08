# Claude Code Configuration

## Performance Optimization Commands

### Network Testing
```bash
# Test network-only performance vs Alloy
cargo run --bin network_only_test --release

# Compare hyper vs reqwest transports
cargo run --bin reqwest_test --release  

# Test raw network performance 
cargo run --bin alloy_network_test --release

# Single request end-to-end test
cargo run --bin single_test --release
```

### Lint and Type Check
```bash
cargo check
cargo clippy
```

## Current Status

**Performance Gap Identified:**
- Target: ≤1100ms network time
- Alloy total: ~1063ms (✅ under target)  
- Raw HTTP calls: ~1705ms (❌ 605ms over target)
- Palantiri hyper: ~1731ms (❌ 631ms over target)
- Palantiri reqwest: ~1771ms (❌ 671ms over target)

**Key Discovery:** Alloy achieves better performance than raw HTTP calls, indicating network-level optimizations beyond parsing. The bottleneck is not Palantiri's transport layer but missing Alloy's network optimizations.

## What We Should Do Next

### Action Plan

1. **Investigate Alloy's HTTP Transport**
   - Find Alloy's actual HTTP client configuration
   - Look for connection pooling, keep-alive settings
   - Check for HTTP/2 vs HTTP/1.1 usage
   - Identify compression or batching optimizations

2. **Reverse Engineer Network Optimizations** 
   - Compare Alloy's request headers vs raw HTTP
   - Check for connection reuse patterns
   - Look for RPC-specific networking tricks
   - Analyze timing of connection establishment vs request/response

3. **Implement Missing Optimizations**
   - Apply Alloy's HTTP client configuration to Palantiri
   - Add any missing headers or connection settings
   - Implement connection pooling improvements
   - Test each optimization incrementally

4. **Validate Performance**
   - Target: Get Palantiri network time ≤1100ms
   - Success metric: Match or beat Alloy's total performance
   - Keep custom parsing advantage (currently ~5ms vs standard)

### Why This Approach Will Work
- Custom parsing already provides 2x speedup advantage
- Network is the only remaining bottleneck  
- Alloy proves sub-1100ms network time is achievable
- Just need to copy their networking approach

## Architecture Notes

- Custom zero-copy parsing provides significant advantage (~5ms vs standard parsing)
- Network layer is the bottleneck, not parsing
- Both hyper and reqwest clients perform similarly (~40ms difference)
- Need to focus on protocol-level optimizations rather than HTTP client choice
