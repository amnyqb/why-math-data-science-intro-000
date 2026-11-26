# Answer keys

One `answer_key.yaml` per case (`b2-1-workforce-inventory.yaml`, `b2-2-market-growth.yaml`, …), written **before** any function is run on that case.

Each entry:

```yaml
- case: B2-2
  finding: one sentence
  type: dominant_loop | leverage_parameter | core_structure | policy | reduction_size | behaviour
  phase: time window, if the paper states one
  value: only if the paper states a number
  qualitative: true | false
  digitised: true      # only if read off a figure
  source: author, year, page, table or figure
  version: the model version the paper used
```

Rules (brief, section 4):

- Only what the paper states. A value read off a plot is marked `digitised: true`.
- Where papers disagree, record both and score against each.
- Obtain the full text. An abstract page gives no numbers.
- **No entry is written from memory.** Every entry cites a page, table or figure of a paper that has been read.

Status: no key is written yet.
