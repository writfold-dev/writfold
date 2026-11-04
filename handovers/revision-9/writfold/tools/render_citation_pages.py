"""Six front-of-atlas pages for the proof-first citation milestone."""
from reportlab.lib.colors import HexColor


def draw_pages(c, base, para, finish, color):
    def card(x, y, w, h, heading, body, accent=False, size=14):
        c.setFillColor(color("pale") if accent else HexColor("#FFFFFF"))
        c.setStrokeColor(color("line"))
        c.roundRect(x, y, w, h, 12, fill=1, stroke=1)
        end = para(heading, x+18, y+h-18, w-36, 17, 22, bold=True)
        end = para(body, x+18, end-10, w-36, size, size*1.44, fill="muted")
        assert end >= y+10, (heading, end, y)

    def row(y, h, left, right, split=270):
        c.setFillColor(HexColor("#FFFFFF"))
        c.roundRect(40, y-h, 1040, h-5, 9, fill=1, stroke=0)
        a = para(left, 57, y-15, split-28, 14, 19, bold=True, fill="teal")
        b = para(right, 55+split, y-15, 1004-split, 14, 19, fill="ink")
        assert min(a,b) >= y-h+9, (left,a,b,y-h)
        return y-h-6

    def arrow(points):
        c.setStrokeColor(color("teal"))
        c.setLineWidth(2)
        path=c.beginPath()
        path.moveTo(*points[0])
        for xy in points[1:]: path.lineTo(*xy)
        c.drawPath(path)
        x,y=points[-2]
        u,v=points[-1]
        tip=c.beginPath()
        tip.moveTo(u,v)
        if x==u:
            d=1 if v>y else -1
            tip.lineTo(u-5,v-d*8)
            tip.lineTo(u+5,v-d*8)
        else:
            d=1 if u>x else -1
            tip.lineTo(u-d*8,v-5)
            tip.lineTo(u-d*8,v+5)
        tip.close()
        c.setFillColor(color("teal"))
        c.drawPath(tip,fill=1,stroke=0)

    base("REVISION 6 / WORK QUEUE", "What the implementing agent should do first",
         "Resume verified work from its actual commit. The full release inventory is available when a dependency calls for it.")
    y=577
    for a,b in [
        ("1   NOW   /   P03C", "Reconcile the existing proof work. Define citation binding and admission, then prove CIT01-CIT08. Keep proposals in candidate state while proofs are missing."),
        ("2   NEXT   /   P05C", "Verify the production Rust checker and build the compact reader. Reproduce the copied-line failure and compare token cost with targeted shell reads."),
        ("3   THEN   /   P07 onward", "Enable each qualified kind after its Lean rule and exact instance certificate pass. Recheck authority and protocol evidence at admission."),
        ("4   LATER   /   experiments", "Add acquisition hints and focused Obsidian explanations. Try a read-only run monitor after ledger evidence is available."),
        ("5   PARKED", "Reconsider heterogeneous TTLs, a standalone graph browser and automated learning rules only when a measured use case justifies them.")]:
        y=row(y,81,a,b)
    para("Daily handoff: one active obligation, its owner, the missing evidence and the next reproducible command. See PRIORITIES.md.",45,119,1030,14,21)
    finish()

    base("REVISION 6 / ADMISSION", "Every accepted proposal has a proof-backed rule",
         "Drafting, experiments and review can proceed before admission. The authoritative transition has a separate gate.")
    card(40,401,320,170,"Trusted profile","The released engine selects the required Lean theorems, proof bundle and qualified implementation for this proposal kind.",True)
    card(400,401,320,170,"Concrete proposal","The Lean worker checks this candidate and witness at the exact predecessor. Its authenticated instance receipt names the evidence and intended use.")
    card(760,401,320,170,"Live conditions","Check policy, grants, freshness, gate receipts and independent review where required.")
    card(400,160,320,168,"Admission decision","Require the rule bundle and instance receipt. Recheck current premises at the authoritative boundary.",True)
    card(40,160,320,168,"Blocked","Missing or stale proof, unsupported kind or invalid witness. Keep selected state unchanged.")
    card(760,160,320,168,"Conditional publication","A successful decision permits the applicable design transition or release CAS. Retain the receipt.")
    arrow([(200,401),(200,362),(486,362),(486,328)])
    arrow([(560,401),(560,328)])
    arrow([(920,401),(920,362),(634,362),(634,328)])
    arrow([(400,244),(360,244)])
    arrow([(720,244),(760,244)])
    para("A new semantic rule must qualify under the old trusted floor and independently prove its successor. The candidate cannot select its own proof requirements.",45,118,1030,14,21)
    finish()

    base("REVISION 6 / CITATION READER", "Four compact operations over retained evidence",
         "Agents exchange short scoped handles. The engine retains the complete source, selector and provenance binding.")
    y=566
    for a,b in [
        ("find", "Discover candidates at a pinned release and named context. Show search coverage, roles and omissions alongside each scoped handle."),
        ("peek", "Expand an authorized handle to the exact retained excerpt. Display bounds, provenance and required caveats within the budget."),
        ("cite", "Expand the canonical public citation and prepare the proposed binding. Admission still requires the qualified rule and witness."),
        ("compare", "Compare an explicit target revision. Report justified correspondence, multiple candidates, proved deletion/replacement or unknown.")]:
        y=row(y,89,a,b,212)
    para("Coordinates belong to a representation",43,176,1030,18,25,bold=True)
    para("A byte interval in a Git blob and a UTF-16 range in a Google Docs tab use different types. A quote hash helps discovery. Identical text can appear twice.",43,137,1028,14,21,fill="muted")
    finish()

    base("REVISION 6 / FIRST REGRESSION", "The copied line is the wrong successor",
         "Reproduced with the actual kctx constructor and current-location classifier in the archived 1c17b94 source.")
    for x,title,lines in [(40,"Original revision",["1  def intended():","2      return DEFAULT"]),
                          (580,"New revision",["1  def other():","2      return DEFAULT","3","4  def intended():","5      return DEFAULT"])]:
        c.setFillColor(color("navy"))
        c.roundRect(x,350,500,215,12,fill=1,stroke=0)
        para(title,x+18,544,465,16,22,bold=True,fill="mint")
        for j,line in enumerate(lines):
            c.setFillColor(color("pale"))
            c.setFont("Mono",15)
            c.drawString(x+20,486-24*j,line)
    card(40,142,500,158,"Observed legacy advisory","The normalized text matches lines 2 and 5. The helper reports moved at line 2, inside other(). The historical source remains intact.")
    card(580,142,500,158,"Required revision-6 result","Expose both candidates. Selecting the intended successor requires additional correspondence evidence. Preserve the historical binding.",True)
    para("Run tools/reproduce_duplicate_citation.py. This reproduces a legacy observation. CIT03 and CIT04 still require checked Lean proofs.",45,99,1030,13,19)
    finish()

    base("REVISION 6 / FRESHNESS", "Three different clocks for the same asset",
         "Choose a policy for the intended decision. A source medium alone says little about how quickly its claims can become stale.")
    card(40,360,320,205,"Recheck scheduling","The catalog names the detector, owner, last successful source check, and polling or event hint. These help choose acquisition work.")
    card(400,360,320,205,"Evidence freshness","The signed context policy assesses the original observation dependencies at an explicit time. An ordinary reread preserves their ages.",True)
    card(760,360,320,205,"Retention","Release, citation, pending-operation and reader roots govern which bytes remain available. Expiry alone cannot destroy history.")
    y=314
    y=row(y,75,"Historical design","A retained source shows what was written at that revision. Its applicability to today's rollout is a separate judgment.")
    y=row(y,75,"Current deployment","A document can stay byte-identical while the deployed version changes. Obtain evidence for the actual target and intended use.")
    para("First use existing named contexts. Different TTLs inside a single witness would change semantics and require a new proved policy extension.",45,120,1030,14,21)
    finish()

    base("REVISION 6 / EVALUATION", "What would justify the next feature?",
         "Record small, falsifiable decisions before expanding the release. Earlier proof gates continue to govern proposal admission.")
    y=570
    for a,b in [
        ("Compact citations","Compare repeated inspect-and-cite tasks with kctx and targeted rg. Count real input/output tokens, retries, latency, wrong attachments and omissions."),
        ("The starting measurement","One fixture returned 4,620 whole-file bytes, 199 targeted rg bytes and 361 bytes of illustrative JSON. The first lookup alone favors rg."),
        ("Focused Obsidian views","Ask whether Tarek can trace a blocker to its test and citation. Show one scoped diagram with explicit edge meanings and an ordinary table."),
        ("Read-only Ratatui pilot","Pages 57-63 show the approved run views. Compare blocker, revision and next-evidence lookup against plain CLI output on the same run."),
        ("Decision","Promote a successful experiment through the normal proof and review path. Keep an inconclusive result provisional and record its evidence.")]:
        y=row(y,79,a,b,277)
    para("The 27-stage release plan provides traceability. The immediate working queue has one mathematical foundation and one checked reader.",45,111,1030,14,21)
    finish()
