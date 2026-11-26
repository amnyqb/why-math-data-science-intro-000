"""The budget Sankey (plots.budget) on an open epidemic toy model: where do the people end up with and without distancing?
Illustrative parameters, not data. Build the model first:
  dyn convert book/examples/charting/epidemic_open.mdl /tmp/epi.sdm
then, from the repo root:  FC_BIN=target/release/dyn PYTHONPATH=python python book/examples/charting/budget.py"""
import dynamist as fc
from dynamist import plots

m = fc.Model("/tmp/epi.sdm")
plots.budget(
    m, [("No distancing", {}), ("With distancing", {"contact rate": 0.30})],
    channels={"Died of the disease": ["disease deaths"],
              "Natural deaths": ["natural death S", "natural death E", "natural death I", "natural death R"],
              "Still susceptible": ["end:Susceptible"], "Immune (recovered)": ["end:Recovered"],
              "Sick (exposed or infectious)": ["end:Exposed", "end:Infectious"]},
    groups={"Died": ("loss", ["Died of the disease", "Natural deaths"]),
            "Alive at the end": ("good", ["Still susceptible", "Immune (recovered)", "Sick (exposed or infectious)"])},
    sources={"start": "People at the start", "births": "Born during the two years"}, unit="people", headline="Alive at the end",
    title="Distancing cuts disease deaths but leaves more people susceptible",
    source="open epidemic toy model with illustrative parameters, not data; 730 days; shares of everyone who was in the system at the start or was born during the period", out="wiki/img/charting/18_budget.png")
