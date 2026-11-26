#!/bin/bash
# Rebuild the book example figures from the .mdl files:  FC=path/to/dyn ./build.sh
# Each chNN/ gets model.sdm (the saved layout), diagram.svg, chart.svg and, where the model has
# feedback loops, loops.svg. The tests (cli/tests/figures.rs) render the same files and compare.
set -euo pipefail
cd "$(dirname "$0")"
FC=${FC:-dyn}
STYLE=book.style.json

# one chapter: DIR MDL "caption" "source" "chart vars" [T0 T1]
chapter() {
  local dir=$1 mdl=$2 cap=$3 src=$4 vars=$5
  mkdir -p "$dir"
  $FC convert "$mdl" "$dir/model.sdm" > /dev/null
  $FC layout "$dir/model.sdm" --mode tidy > /dev/null
  $FC set "$dir/model.sdm" --caption "$cap" --source "$src" > /dev/null
  $FC style "$dir/model.sdm" --apply "$STYLE" > /dev/null
}

render() { # DIR SHEET OUTNAME
  $FC render "$1/model.sdm" --sheet "$2" --out "$1/$3.svg" --with-caption > /dev/null
}

# place words by hand: FILE then NAME X Y triples
place() { # FILE "NAME" X Y ["NAME" X Y]...
  local f=$1; shift
  local args=()
  while [ $# -gt 0 ]; do args+=(--pos "$1" "$2" "$3"); shift 3; done
  $FC set "$f" "${args[@]}" > /dev/null
}

# ---- ch10 shipment pipeline
chapter ch10 ../examples/ch10-shipment-pipeline.mdl "Figure 10.1  One order stream, four ways to model the delay." "Chapter 10 example model" "Orders,Smooth arrival,Bunched arrival,Pipeline arrival"
place ch10/model.sdm "Orders" 100 140 "Smooth arrival" 360 40 "Bunched arrival" 360 120 "Pipeline arrival" 360 200 "Belief about orders" 360 280
render ch10 Model diagram
$FC chart ch10/model.sdm --vars "Orders,Smooth arrival,Bunched arrival,Pipeline arrival" --out ch10/chart.svg --title "Orders and three arrival patterns" > /dev/null

# ---- ch11 emergency room
chapter ch11 ../examples/ch11-emergency-room.mdl "Figure 11.1  The emergency room: arrivals, treatment and the waiting line." "Chapter 11 example model" "Arrivals,Treated,Waiting"
$FC set ch11/model.sdm --polarity Treated Waiting - --polarity Waiting Treated + > /dev/null
place ch11/model.sdm "Arrivals" 100 110 "Treatment capacity" 100 230 "Treated" 330 170 "Waiting" 560 170 "Average wait" 800 70 "Longest wait" 800 150 "Waiting over 2 hours" 800 230
render ch11 Model diagram
$FC chart ch11/model.sdm --vars "Waiting,Waiting over 2 hours" --out ch11/chart.svg --title "Who is waiting" > /dev/null
$FC cld ch11/model.sdm --from-model --out ch11/model.sdm --layout loops > /dev/null
$FC set ch11/model.sdm --sheet "Causal loops" --loop-name B1 "Queue clears" --caption "Figure 11.2  The line balances itself: the longer it is, the more is treated." > /dev/null
render ch11 "Causal loops" loops

# ---- ch12 water shortage
chapter ch12 ../examples/ch12-water-shortage.mdl "Figure 12.1  Three ways to share the water." "Chapter 12 example model" "Supply"
place ch12/model.sdm "Demand" 100 60 "Priority" 100 160 "Supply" 100 260 "Equal share" 380 60 "By priority" 380 160 "Softened priority" 380 260
render ch12 Model diagram
$FC chart ch12/model.sdm --vars "Equal share[North],By priority[North],Softened priority[North]" --out ch12/chart.svg --title "North's share under three rules" > /dev/null

# ---- ch13 ageing population
chapter ch13 ../examples/ch13-ageing-population.mdl "Figure 13.1  Population by age group." "Chapter 13 example model" "Old share"
place ch13/model.sdm "Start population" 150 60 "Mortality" 400 60 "Fertility" 650 60 "Change" 180 200 "Population" 420 200 "Births per year" 650 200 "Total" 420 340 "Old share" 650 340
render ch13 Model diagram
$FC chart ch13/model.sdm --vars "Old share" --out ch13/chart.svg --title "Share aged 65 or more" > /dev/null
$FC cld ch13/model.sdm --from-model --out ch13/model.sdm --layout loops > /dev/null
$FC set ch13/model.sdm --sheet "Causal loops" --caption "Figure 13.2  Births and ageing feed the population." > /dev/null
render ch13 "Causal loops" loops

# ---- ch14 investment
chapter ch14 ../examples/ch14-investment.mdl "Figure 14.1  Cash flow and present value." "Chapter 14 example model" "Cash flow"
place ch14/model.sdm "Cash flow" 100 100 "Present value" 360 100
render ch14 Model diagram
$FC chart ch14/model.sdm --vars "Cash flow,Present value" --out ch14/chart.svg --title "Cash flow and its present value" > /dev/null

# ---- ch15 beliefs
chapter ch15 ../examples/ch15-beliefs.mdl "Figure 15.1  What a manager believes about demand." "Chapter 15 example model" "Demand"
place ch15/model.sdm "Demand" 100 140 "Perceived demand" 360 40 "Perceived trend" 360 140 "Expected demand in 8 weeks" 360 240
render ch15 Model diagram
$FC chart ch15/model.sdm --vars "Demand,Perceived demand,Expected demand in 8 weeks" --from 0 --to 40 --out ch15/chart.svg --title "Demand and what is believed about it" > /dev/null
echo "done"
