# DQN port (P3 / R2)

Source: github.com/OptMLGroup/DeepBeerInventory-RL (BSD 3-Clause), the code of Oroojlooyjadid, Nazari, Snyder and
Takac, "A Deep Q-Network for the Beer Game" (M&SOM 2022). `orig/` holds the unchanged files; `port/` the PyTorch version.

What changed: only `SRDQN.py` (TensorFlow 1.15 graph code replaced by a PyTorch module with the same interface) and the
TensorFlow imports/logging calls in `main.py`, `clBeergame.py`, `utilities.py`. The game, the agents, the Sterman and
base-stock co-players, the demand files and the config are the authors' code, untouched.

Deliberate differences in SRDQN: no transfer learning, TensorBoard or TF checkpoints (a state_dict is saved);
ring-buffer replay (same behaviour, O(1) sampling); learning-rate schedule, initialisation, Adam settings, target-network
update and epsilon schedule reproduced from the original. Not checked against the authors' numbers beyond R2's sign test.

Runs: Colab T4 through `colab` CLI. Demand: `--demandDistribution 0 --demandUp 9` is U{0..8} (numpy's high is exclusive), the
paper's U[0,8]. Scenarios: gameConfig 3 = DQN retailer with base-stock co-players (S1), 7 = DQN retailer with Sterman co-players (S2).
