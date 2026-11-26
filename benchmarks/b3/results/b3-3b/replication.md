# B3-3b replication: Dynamist World3 (1974) against PyWorld3

Model: `benchmarks/b3/models/build_world3_1974.py` (equations and their PyWorld3 file:line in `W3_1974_SOURCES.md`). Reference: PyWorld3 standard run (`World3()` defaults, `run_world3(fast=False)`, year 1900-2100), exported by `run_pyworld3.py` to `pyworld3_standard_dt*.csv` in this folder. Both sides are explicit Euler at the same dt.

**Tolerance, fixed before the comparison: 1% relative difference on population, industrial output, food, persistent pollution index and nonrenewable resources fraction remaining at 1950, 2000, 2050 and 2100.**

Names: POP population, IO industrial output, F food, PPOLX persistent pollution index, NRFR nonrenewable resources fraction remaining.

## Main result (dt = 0.5, PyWorld3's own default step)

### default model vs PyWorld3, dt 0.5

steps compared: 401 (Dynamist 401, PyWorld3 401); tolerance 1%

| variable | year | PyWorld3 | Dynamist | relative difference |
|---|---|---|---|---|
| population (POP) | 1950 | 2.6074e+09 | 2.60276e+09 | 0.178% |
| population (POP) | 2000 | 5.64348e+09 | 5.63473e+09 | 0.155% |
| population (POP) | 2050 | 6.2376e+09 | 6.24528e+09 | 0.123% |
| population (POP) | 2100 | 3.97686e+09 | 3.98058e+09 | 0.094% |
| industrial output (IO) | 1950 | 3.96324e+11 | 3.94497e+11 | 0.461% |
| industrial output (IO) | 2000 | 1.98509e+12 | 1.97839e+12 | 0.338% |
| industrial output (IO) | 2050 | 5.57523e+11 | 5.61007e+11 | 0.625% |
| industrial output (IO) | 2100 | 4.25949e+10 | 4.28925e+10 | 0.699% |
| food (F) | 1950 | 1.0384e+12 | 1.03544e+12 | 0.284% |
| food (F) | 2000 | 2.79422e+12 | 2.78875e+12 | 0.196% |
| food (F) | 2050 | 1.6004e+12 | 1.60508e+12 | 0.293% |
| food (F) | 2100 | 9.37532e+11 | 9.37851e+11 | 0.034% |
| persistent pollution index (PPOLX) | 1950 | 0.487592 | 0.489812 | 0.455% |
| persistent pollution index (PPOLX) | 2000 | 3.5301 | 3.51126 | 0.534% |
| persistent pollution index (PPOLX) | 2050 | 7.59205 | 7.62997 | 0.499% |
| persistent pollution index (PPOLX) | 2100 | 0.601865 | 0.605835 | 0.660% |
| nonrenewable resources fraction remaining (NRFR) | 1950 | 0.961285 | 0.961431 | 0.015% |
| nonrenewable resources fraction remaining (NRFR) | 2000 | 0.688022 | 0.689508 | 0.216% |
| nonrenewable resources fraction remaining (NRFR) | 2050 | 0.196045 | 0.196347 | 0.154% |
| nonrenewable resources fraction remaining (NRFR) | 2100 | 0.152024 | 0.152048 | 0.016% |

largest difference at the 20 checked points: 0.699% (within tolerance)

| variable | max relative difference over 1900-2100 | at year | last year with a difference above tolerance |
|---|---|---|---|
| population (POP) | 0.206% | 1967.0 | never |
| industrial output (IO) | 1.681% | 1900.0 | 1900.0 |
| food (F) | 4.956% | 1900.0 | 1900.0 |
| persistent pollution index (PPOLX) | 428.234% | 1906.5 | 1946.5 |
| nonrenewable resources fraction remaining (NRFR) | 0.513% | 2019.0 | never |

## The same with PyWorld3's own DELAY3 start (variant, used to find the cause of the early pollution difference)

### variant with PyWorld3 DELAY3 start vs PyWorld3, dt 0.5

steps compared: 401 (Dynamist 401, PyWorld3 401); tolerance 1%

| variable | year | PyWorld3 | Dynamist | relative difference |
|---|---|---|---|---|
| population (POP) | 1950 | 2.6074e+09 | 2.60755e+09 | 0.006% |
| population (POP) | 2000 | 5.64348e+09 | 5.64368e+09 | 0.004% |
| population (POP) | 2050 | 6.2376e+09 | 6.23844e+09 | 0.013% |
| population (POP) | 2100 | 3.97686e+09 | 3.97724e+09 | 0.010% |
| industrial output (IO) | 1950 | 3.96324e+11 | 3.96267e+11 | 0.014% |
| industrial output (IO) | 2000 | 1.98509e+12 | 1.98483e+12 | 0.013% |
| industrial output (IO) | 2050 | 5.57523e+11 | 5.57698e+11 | 0.031% |
| industrial output (IO) | 2100 | 4.25949e+10 | 4.26113e+10 | 0.038% |
| food (F) | 1950 | 1.0384e+12 | 1.03835e+12 | 0.004% |
| food (F) | 2000 | 2.79422e+12 | 2.79404e+12 | 0.007% |
| food (F) | 2050 | 1.6004e+12 | 1.60063e+12 | 0.014% |
| food (F) | 2100 | 9.37532e+11 | 9.37562e+11 | 0.003% |
| persistent pollution index (PPOLX) | 1950 | 0.487592 | 0.48769 | 0.020% |
| persistent pollution index (PPOLX) | 2000 | 3.5301 | 3.5294 | 0.020% |
| persistent pollution index (PPOLX) | 2050 | 7.59205 | 7.59406 | 0.026% |
| persistent pollution index (PPOLX) | 2100 | 0.601865 | 0.602092 | 0.038% |
| nonrenewable resources fraction remaining (NRFR) | 1950 | 0.961285 | 0.961287 | 0.000% |
| nonrenewable resources fraction remaining (NRFR) | 2000 | 0.688022 | 0.688083 | 0.009% |
| nonrenewable resources fraction remaining (NRFR) | 2050 | 0.196045 | 0.196065 | 0.010% |
| nonrenewable resources fraction remaining (NRFR) | 2100 | 0.152024 | 0.152028 | 0.003% |

largest difference at the 20 checked points: 0.038% (within tolerance)

| variable | max relative difference over 1900-2100 | at year | last year with a difference above tolerance |
|---|---|---|---|
| population (POP) | 0.172% | 1904.0 | never |
| industrial output (IO) | 1.681% | 1900.0 | 1900.0 |
| food (F) | 4.956% | 1900.0 | 1900.0 |
| persistent pollution index (PPOLX) | 6.987% | 1906.5 | 1921.0 |
| nonrenewable resources fraction remaining (NRFR) | 0.024% | 2019.0 | never |

| variable | max rel. difference | at year | value at 1900 PyWorld3 | Dynamist |
|---|---|---|---|---|
| ppol | 6.987% | 1906.5 | 2.5e+07 | 2.5e+07 |
| ppgr | 9.901% | 1900.0 | 1.04525e+07 | 1.14874e+07 |
| ppapr | 9.901% | 1900.0 | 1.56788e+06 | 1.7231e+06 |
| io | 1.681% | 1900.0 | 6.65e+10 | 6.53822e+10 |
| iopc | 1.681% | 1900.0 | 41.5625 | 40.8639 |
| fpc | 4.956% | 1900.0 | 269.325 | 282.672 |
| f | 4.956% | 1900.0 | 4.3092e+11 | 4.52275e+11 |
| le | 1.036% | 1900.0 | 28.0437 | 28.3342 |
| ai | 3.080% | 1900.5 | 6.21013e+09 | 6.21013e+09 |
| pfr | 1.239% | 1900.5 | 1.17098 | 1.17098 |
| cuf | 0.452% | 1900.5 | 0.983191 | 0.983191 |
| ic | 0.048% | 1903.0 | 2.1e+11 | 2.1e+11 |

## Summary over variants and time steps

Worst relative difference at the 20 checkpoints, and worst difference over 1930-2100 for each main variable (1900-1930 holds the start-up transient described in the next section).

| Dynamist variant, dt | worst at checkpoints | POP | IO | F | PPOLX | NRFR |
|---|---|---|---|---|---|---|
| default, dt 0.5 | 0.699% | 0.206% | 0.716% | 0.414% | 14.340% | 0.513% |
| PyWorld3 DELAY3 start, dt 0.5 | 0.038% | 0.026% | 0.038% | 0.021% | 0.323% | 0.024% |
| book start LUFD/AI/PFR, dt 0.5 | 0.580% | 0.206% | 0.603% | 0.358% | 12.580% | 0.436% |
| book start + PyWorld3 DELAY3 start, dt 0.5 | 0.031% | 0.026% | 0.022% | 0.014% | 0.174% | 0.010% |
| default, dt 0.25 | 0.741% | 0.222% | 0.750% | 0.439% | 14.647% | 0.539% |
| PyWorld3 DELAY3 start, dt 0.25 | 0.074% | 0.018% | 0.074% | 0.043% | 0.270% | 0.051% |
| book start LUFD/AI/PFR, dt 0.25 | 0.604% | 0.216% | 0.619% | 0.362% | 12.864% | 0.448% |
| book start + PyWorld3 DELAY3 start, dt 0.25 | 0.051% | 0.036% | 0.030% | 0.024% | 0.217% | 0.005% |
| default, dt 1 | 0.608% | 0.175% | 0.626% | 0.372% | 13.672% | 0.446% |
| PyWorld3 DELAY3 start, dt 1 | 0.086% | 0.048% | 0.061% | 0.040% | 0.431% | 0.045% |
| book start LUFD/AI/PFR, dt 1 | 0.534% | 0.187% | 0.559% | 0.341% | 11.961% | 0.402% |
| book start + PyWorld3 DELAY3 start, dt 1 | 0.068% | 0.023% | 0.068% | 0.036% | 0.087% | 0.045% |

## Where the schemes differ, and the cause of every difference above

Both programs step explicitly with Euler: every state is updated from the rates of the previous step, then every auxiliary is evaluated at the new time. PyWorld3 (`world3.py` `_run_world3`, loop over `k`; rates are stored at index `kl = k`, states read `[jk] = [k-1]`) and Dynamist (`Euler`, all variables in dependency order, then stocks by `dt x flows`) are the same scheme, so there is no integration difference and the two agree at dt = 1, 0.5 and 0.25 alike. The time grid is the same (1900 to 2100 inclusive, 401 points at dt = 0.5).

The smooths and delays map one to one:

| PyWorld3 (`specials.py`) | Dynamist | Start value |
|---|---|---|
| `Smooth` (:128), euler: `out[k] = out[k-1] + dt/delay (in[k-1] - out[k-1])` | `SMOOTH` | input at k=0 (the same) |
| `Dlinf3` (:261), three stages of `delay/3`, output stage 3 | `SMOOTH3` | all stages = input at k=0 (the same) |
| `Delay3` (:199), same stages, output stage 3 | `DELAY3` | **different**, see 1 |

What differs, all in the first step or so of the run (none touches the equations):

1. **DELAY3 start (PPAPR = DELAY3(PPGR, PPTD)).** `Delay3._init_out_arr` (`specials.py:241-242`) starts every stage at `in * 3 / delay`, so the outflow at 1900 is 0.15 x PPGR (delay 20) instead of PPGR. A DYNAMO or Dynamist DELAY3 starts at the input. Effect: PyWorld3's pollution stock first falls (absorption exceeds the small inflow) where Dynamist's rises. The persistent pollution index differs by up to several hundred percent around 1906 when the index is still below 0.2, and is back within 1% from 1947 on. The variant `--delay3-init pw3` (`DELAY3I(PPGR, PPTD, PPGR*3/PPTD)`) reproduces PyWorld3: worst checkpoint difference 0.04%.
2. **Initial-value loops.** Three smooths sit in a loop of initial values (LUFD in CUF-IO-IOPC-JPICU-PJIS-J-LUF; AI in AI-AIPH-LYMC-LY-MPLD-FIALD-CAI; PFR in PFR-FALM-AIPH-LYMC-LY-F-FPC-FR). Dynamist needs a start value for each (`SMOOTHI`). PyWorld3 resolves them by repeated passes at k=0 in which `cuf[0]=1`, `ai[0]=5e9` and `pfr[0]=1` are preset (`capital.py:392`, `agriculture.py:474-475`); the values it stores at k=0 (lufd 1.336, ai 6.21e9, pfr 1.171) are the inputs of the last pass, not the values used in that pass. Effect: PyWorld3's k=0 row is not a consistent state (for example IO[0] is computed with CUF=1 but CUF[0] is stored as 0.983) and its rates at 1900 differ by 1.7% (IO) to 5% (food per capita) from a consistent evaluation. The default build starts the three smooths at the values PyWorld3 stores (`--smooth-init pw3`), so the smoothed states agree from the second step on; with the book's own start values (`--smooth-init book`: LUFD=1, AI=5e9, PFR=1) the early states differ for a few years (smoothing times 2 years) and the checkpoints stay within 0.6%.
3. **Lagged deaths.** `d[k]` is the sum of the deaths of step k-1 (`population.py:800`), and `cbr[k]` uses `b[k-1]`. Nothing depends on them in the standard run (birth rate switches to deaths only after PET = 4000), so `D`, `CDR`, `CBR` in the Dynamist model are the current-step values and carry no weight; the `B` equation leaves out the PET branch.

PyWorld3 versus the 1974 book is a separate question that this replication does not settle: the Dynamist model is transcribed from PyWorld3, so any place where PyWorld3 departs from the book is inherited. Visible in the code: `FIOAC` stays at its constant (`IET = 4000`), `PPGF` is a constant 1, and `LUFD` is a first-order smooth of `LUF`.

