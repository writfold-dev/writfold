# What a good reflection covers

Write it as one checkpoint entry whose first line starts `reflection:`,
then attest this gate. The `writfold:evaluate` skill walks these steps. A
good reflection is short and specific to this run, and covers four things:

- **What was done.** What changed, and whether it did what the task asked.
  Name the commits or the files, and the gates or citations the result
  rests on; say which were waived or skipped, and why.
- **What was learned.** What took longer or went differently from the
  plan, and why; a fact about the code or the project that was not
  written down before. "Nothing" is an answer, if it is true.
- **What should change.** One change to the plan template, a gate, the
  code or the KB, if any. A change to the KB is a proposal
  (`writfold kb propose …`, with citations), not an edit made now.
- **What was backfilled.** The KB records written or proposed during the
  run, by ID. If a question came up that the KB could not answer and
  nothing was recorded, propose the record that would have answered it,
  or write a line starting `no proposal:` with the reason.

Do not restate the diff, and do not grade the work: say what a reader
picking up the next run needs to know.
