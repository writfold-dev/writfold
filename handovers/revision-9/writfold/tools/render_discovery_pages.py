"""Three design-review pages about incident discovery. Rendered after Run Watch."""
def draw_pages(c, base, para, finish, color):
    def row(y, tag, title, body):
        para(tag,40,y,134,12,18,fill='teal',bold=True)
        para(title,190,y+2,344,18,24,bold=True)
        para(body,552,y+1,520,15,21)
        c.setStrokeColor(color('line')); c.line(40,y-78,1080,y-78)
    base('SIMILAR CASES  /  INVESTIGATION','Find the earlier incident',
         'Synthetic example. Older SDK clients fail after an API rollout. No ticket number or exact error string is known.')
    c.setFillColor(color('navy'));c.roundRect(40,440,1040,140,12,fill=1,stroke=0)
    para('AGENT ASKS',64,553,935,12,bold=True,fill='mint')
    para('“Have older callers failed when a service added a response field?”',64,513,962,21,28,bold=True,fill='pale')
    para('Scope: checkout  /  corpus R42  /  budget: three case handles',64,468,950,14,19,fill='mint')
    row(408,'01   FIND','Unknown-field exception','A historical SDK case mentions a strict decoder. A retry-loop report also matched the endpoint.')
    row(304,'02   PEEK','Read the discriminator','Open the retained evidence. In the old incident the decoder threw before the next-page request.')
    row(200,'03   TEST HERE','Check today’s failing client','Look for the exception at the affected version. If requests succeed and tokens repeat, investigate the retry loop instead.')
    para('The historical fix remains a hypothesis. Cite the old evidence and the new observation separately.',40,86,1020,14,19,fill='muted')
    finish()
    base('SIMILAR CASES  /  VERIFICATION','What a match can establish',
         'The ranker may be approximate, incomplete or hostile. A handle still has to resolve inside the pinned, authorized context.')
    row(560,'PROVED','Search leaves authority unchanged','An independent checker verified trace erasure by induction for arbitrarily long finite traces, plus five guard laws. Deliberately broken proof cases failed.')
    row(458,'EXPLORED','276 states and 1,640 edges','The finite Python model found zero violations in the guarded variant. Removing each of three checks produced a counterexample.')
    row(356,'PENDING','Lean and protocol gates','The packet contains Lean proof terms and a TLA+ model. Lean kernel, TLC, TLAPS and Rust refinement receipts remain open.')
    row(254,'MEASURE','Do the right cases surface?','Compare a skilled grep workflow with lexical and hybrid retrieval. Count tokens through the final defensible citation, including failed investigations.')
    para('An empty result can coexist with a real incident. A visible result can still have a different cause.',40,112,1030,15,21,fill='teal',bold=True)
    finish()
    base('OMNIGRAPH  /  CAPABILITY FIT','Where the graph earns its place',
         'Review of the 0.14.0 source pin. Promote a capability only when it answers a concrete Writfold question.')
    row(560,'PILOT','Search inside relationships','Combine lexical and vector ranking with dependency traversal. A correlated evidence query can distinguish an observed symptom from a passing mention.')
    row(458,'USE','Named, bounded queries','Bind one reviewed query and scope to find. Keep graph syntax, limits, revision selection and admission authority in the engine.')
    row(356,'QUALIFY','Explain plans and snapshots','Inspect query plans for broad scans. Replay the pinned source and retained hit list. Consider native diff later if it reduces rebuild cost.')
    row(254,'DEFER','More agent write surfaces','Branches per agent, generic mutating MCP and a separate vector database add no direct value to the first incident workflow.')
    para('Enable hybrid only if it improves top-three recall and lowers tokens per correct investigation without extra unsafe transfers.',40,116,1015,15,22,fill='teal',bold=True)
    finish()
