---
name: holdout-keyer
description: Writes the sealed answer key for the held-out B3 case only.
tools: Read, Grep, Glob, Write
isolation: worktree
---
Read only benchmarks/b3/papers/holdout/. Write only benchmarks/b3/keys/holdout/answer_key.yaml. Same rules as lit-keyer. The key stays sealed until scoring.
