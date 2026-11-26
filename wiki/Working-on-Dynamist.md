# Working on Dynamist: repository, staging and tests

**Where the code lives.** The private GitHub repository `amnyqb/dynamist` (default branch `master`). On the main computer the working copy is the clone `dynamist-clean`. The first repository (`app`) is kept as a backup only: it still holds large benchmark result files that GitHub refuses (over 100 MB), so the history in the new repository was rewritten without them (`benchmarks/b3/results/b2-3/` and `results_server/b2-3/`: `importance.json`, `reduce.json`, `reduction_log.csv`). The commit ids therefore differ from the old repository. Those result files are generated and can be rerun.

**GitHub is the staging area.** Heavy work (the test suite, release builds, benchmark sweeps, model sweeps) never runs on the laptop. The laptop pushes a branch; the Contabo server pulls it and runs the work.

```
scripts/server-test.sh                      # push the current branch; whole workspace test on the server
scripts/server-test.sh BRANCH               # another branch
scripts/server-test.sh BRANCH -- test --release -p dynamist-core doctor
```

The server keeps one clone in `/root/dynamist` (a read-only deploy key) and one shared build folder. The script prints the result lines and leaves the full log in `/root/dynamist/server-test.log`. On the laptop only a quick `cargo check` or one small test is fine.

**Names.** The project is Dynamist. The command line is `dyn` (it was `fc`), the native model file is `.sdm` (it was `.flowmodel`), the crates are `dynamist`, `dynamist-core` and `dynamist-cli`. Old files keep working: `.flowmodel` opens everywhere `.sdm` does, the old XMILE settings comment (`flowcanvas:meta`) and the old Vensim comment tag (`[flowcanvas: ...]`) are still read, and the old preferences file is read when the new one is absent. Internal names the importers wrote into imported Vensim and Stella data (`fc graph`, `fc lookup`, `fc init`) are kept, because existing files contain them.

**Branches.** `master` has the layers, the Architecture sheet, the rename, the library catalogue and the classics. The enhancement programme (exact units, calendar time, the model doctor, the function library) is on its own branch, `enhancement`, until its tests are green on the server.
