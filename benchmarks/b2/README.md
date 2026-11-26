# B2: published Vensim and Stella models in Dynamist

Goal: a model the user already has, in Vensim `.mdl` or Stella `.stmx`/XMILE, imports into Dynamist, runs without
issues, and gives **the same numbers** as the tool it was built in.

- `run_b2.py` imports every model file of two public test suites with `dyn run`, runs it, and compares every
  variable at every saved time with the output the original tool exported (`results/b2_models.csv`).
- `report_b2.py` also imports and runs every model in a local Vensim install (World3-03, Urban Dynamics, Market
  Growth, ...; run only, there is no reference output for them yet) and writes [B2_report.md](B2_report.md) with the
  fix list.

The downloaded suites live in `external/` and are not committed (see the report for how to fetch them).
