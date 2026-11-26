"""PyTorch port of SRDQN.py from OptMLGroup/DeepBeerInventory-RL (BSD 3-Clause, Lehigh Optimization and
Machine Learning Group, 2020). The class keeps the original interface (setInitState, getDNNAction, train,
replayMemory, epsilon schedule) so that BGAgent.py and clBeergame.py are unchanged apart from removing the
TensorFlow imports. Differences, all deliberate and listed in dqn/PORT_NOTES.md:
  * no transfer learning, no TensorBoard, no TF checkpoints (a PyTorch state_dict is saved instead);
  * replay memory is a ring buffer with O(1) sampling (the original deque makes random.sample O(n));
  * weight init is truncated normal(0, 0.1) and bias 0.1, as in the original; Adam(0.9, 0.999, 1e-8) with
    staircase exponential learning-rate decay as in the original."""
import os, random
import numpy as np
import torch
import torch.nn as nn

DEV = torch.device("cuda" if torch.cuda.is_available() else "cpu")


class Replay:
    """List-like ring: append, popleft, len, and (negative) indexing, which clBeergame uses to edit rewards."""
    def __init__(self):
        self.buf, self.head = [], 0
    def __len__(self):
        return len(self.buf) - self.head
    def append(self, x):
        self.buf.append(x)
    def popleft(self):
        self.head += 1
        if self.head > 100000:
            self.buf = self.buf[self.head:]; self.head = 0
    def __getitem__(self, i):
        return self.buf[self.head + i] if i >= 0 else self.buf[i]
    def sample(self, k):
        n = len(self)
        return [self.buf[self.head + j] for j in random.sample(range(n), k)]


def _net(nodes):
    layers = []
    for a, b in zip(nodes[:-1], nodes[1:]):
        lin = nn.Linear(a, b)
        nn.init.trunc_normal_(lin.weight, std=0.1, a=-0.2, b=0.2)
        nn.init.constant_(lin.bias, 0.1)
        layers += [lin, nn.ReLU()]
    return nn.Sequential(*layers[:-1])  # no activation on the Q output


class DQN:
    def __init__(self, agentNum, config):
        torch.manual_seed(1)
        self.agentNum, self.config = agentNum, config
        self.address = os.path.join(config.model_dir, "model" + str(agentNum + 1))
        self.epsilon = config.epsilonBeg if config.maxEpisodesTrain != 0 else 0
        self.epsilonRed = self.epsilonBuild()
        self.inputSize = config.stateDim * config.multPerdInpt
        self.timeStep = 0
        self.replayMemory, self.replaySize = Replay(), 0
        self.currentState = []
        nodes = list(config.nodes)
        self.Q, self.QT = _net(nodes).to(DEV), _net(nodes).to(DEV)
        self.QT.load_state_dict(self.Q.state_dict())
        self.opt = torch.optim.Adam(self.Q.parameters(), lr=config.lr0, betas=(0.9, 0.999), eps=1e-8)
        self.lr = config.lr0
        self.lrFrozen = False
        self.updates = 0

    # ---- learning rate: staircase exponential decay, frozen once it falls below Minlr (as in the original)
    def _lr(self):
        c = self.config
        if not c.ifDecayAdam or self.lrFrozen:
            return self.lr
        lr = c.lr0 * (c.decayRate ** (self.updates // c.decayStep))
        if lr < c.Minlr:
            self.lrFrozen = True
        self.lr = lr
        return lr

    def trainQNetwork(self):
        c = self.config
        mb = self.replayMemory.sample(c.batchSize)
        s = torch.as_tensor(np.array([m[0] for m in mb]), dtype=torch.float32, device=DEV).reshape(-1, self.inputSize)
        a = torch.as_tensor(np.array([m[1] for m in mb]), dtype=torch.float32, device=DEV)
        r = torch.as_tensor(np.array([m[2] for m in mb], dtype=np.float32), device=DEV)
        s2 = torch.as_tensor(np.array([m[3] for m in mb]), dtype=torch.float32, device=DEV).reshape(-1, self.inputSize)
        term = torch.as_tensor(np.array([m[4] for m in mb], dtype=np.float32), device=DEV)
        with torch.no_grad():
            y = r + (1 - term) * c.gamma * self.QT(s2).max(dim=1).values
        q = (self.Q(s) * a).sum(dim=1)
        loss = ((y - q) ** 2).mean()
        for g in self.opt.param_groups:
            g["lr"] = self._lr()
        self.opt.zero_grad(); loss.backward(); self.opt.step()
        self.updates += 1
        if (self.timeStep + 1) % c.saveInterval == 0:
            os.makedirs(self.address, exist_ok=True)
            torch.save(self.Q.state_dict(), os.path.join(self.address, "network-%d.pt" % self.timeStep))
        if self.timeStep % c.dnnUpCnt == 0:
            self.QT.load_state_dict(self.Q.state_dict())

    def train(self, nextObservation, action, reward, terminal, playType):
        c = self.config
        newState = np.append(self.currentState[1:, :], [nextObservation], axis=0)
        if playType == "train":
            if c.MultiAgent:
                if c.MultiAgentRun[self.agentNum]:
                    self.replayMemory.append([self.currentState, action, reward, newState, terminal]); self.replaySize = len(self.replayMemory)
            else:
                self.replayMemory.append([self.currentState, action, reward, newState, terminal]); self.replaySize = len(self.replayMemory)
            if self.replaySize > c.maxReplayMem and c.MultiAgentRun[self.agentNum]:
                self.replayMemory.popleft(); self.trainQNetwork(); state = "train"; self.timeStep += 1
            elif self.replaySize >= c.minReplayMem and c.MultiAgentRun[self.agentNum]:
                state = "train"; self.trainQNetwork(); self.timeStep += 1
            else:
                state = "observe"
            if terminal and state == "train":
                self.epsilonReduce()
        self.currentState = newState

    def getDNNAction(self, playType):
        c = self.config
        action = np.zeros(c.actionListLen)
        if playType == "train" and (random.random() <= self.epsilon or self.replaySize < c.minReplayMem):
            action[random.randrange(c.actionListLen)] = 1
        elif playType in ("train", "test"):
            with torch.no_grad():
                x = torch.as_tensor(np.array([self.currentState]), dtype=torch.float32, device=DEV).reshape(1, -1)
                action[int(torch.argmax(self.Q(x)[0]).item())] = 1
        return action

    def setInitState(self, observation):
        self.currentState = np.stack([observation for _ in range(self.config.multPerdInpt)], axis=0)

    def epsilonBuild(self):
        if self.config.maxEpisodesTrain != 0:
            return (self.config.epsilonBeg - self.config.epsilonEnd) / (self.config.maxEpisodesTrain * 0.8)
        return 0

    def epsilonReduce(self):
        if self.epsilon > self.config.epsilonEnd:
            self.epsilon -= self.epsilonRed
