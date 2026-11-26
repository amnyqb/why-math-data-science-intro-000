#!/bin/bash
# Rebuild the comparison tables from the stored result files.
#   compare-gemini-failed.md  the files as the runs wrote them (the Gemini judge was out of quota: many rows ungradable)
#   compare-deepseek.md       the same responses re-graded with DeepSeek as the judge (all rows)
cd "$(dirname "$0")/../.."
R=benchmarks/sdai/results
COMMIT=aa39e264df927b621abd4829c096aec937856a88
python3 scripts/sdai-compare.py --sdai-commit $COMMIT --off $R/dev-off-pass1.json.gz $R/dev-off-pass2.json.gz $R/dev-off-pass3.json.gz --on $R/dev-on-pass1.json.gz $R/dev-on-pass2.json.gz $R/dev-on-pass3.json.gz > benchmarks/sdai/compare-gemini-failed.md
python3 scripts/sdai-compare.py --sdai-commit $COMMIT --off $R/dev-off-pass1.regraded.json.gz $R/dev-off-pass2.regraded.json.gz $R/dev-off-pass3.regraded.json.gz --on $R/dev-on-pass1.regraded.json.gz $R/dev-on-pass2.regraded.json.gz $R/dev-on-pass3.regraded.json.gz > benchmarks/sdai/compare-deepseek.md
