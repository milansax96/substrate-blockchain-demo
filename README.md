# substrate-blockchain-demo

A basic implementation of a primitve UXTO Store located in runtime/src/utxo.rs. The objective of this demo is to construct a blockchain implementation, similar to Bitcoin, where transactions are stored as UTXOs, and are validated by different nodes. After validation, the UTXOs are finalized, and then they are emitted to the rest of the blockchain network. In addition, a reward is collected and dispersed, similar to the GAS fee that is required for a transaction to be finalized.

This demo utilizes tutorials from Parity, using the Rust programming language.
