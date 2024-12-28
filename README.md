Light Client Analysis
Type: Ethereum Execution Layer Light Client

Uses block headers only
No state storage
RPC based verification
Uses finality from consensus layer
Key Features:

Block header sync
Finality tracking
Parent hash verification
State root verification
Chain reorganization handling
Components:

Consensus verification
Block sync
RPC client
State management
Finality tracking
Missing Features:

Merkle proof verification
Transaction receipt verification
Account state verification
Consensus layer sync
Validator set management


TODO: SINCE THE HASH IS USED MULTIPLE TIMES, WE SHOULD STORE IT INSTEAD OF MAKING CALLS TO THE RPC FOR EACH INSTANCE USAGE