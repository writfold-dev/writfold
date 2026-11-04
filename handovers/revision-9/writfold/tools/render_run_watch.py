"""Add the human walkthrough and approved terminal views to the end of the atlas."""
from pathlib import Path
import pymupdf

ROOT = Path(__file__).resolve().parents[1]
SCREENS = [
    ("Gate inspector", "Event 34. Three revision-bound checks pass. Two matching citation spans block CP-12."),
    ("Pipeline trace", "The same event as the previous page. Follow the citation branch to see what it holds up."),
    ("Citation inspector", "Open C7 from either view. Both functions contain the same line at the target revision."),
    ("When the reply goes missing", "At event 52 the effect is unknown. Two newer ledger events lie outside the displayed cut."),
    ("Confirming the PR by lookup", "At event 55 an independent forge observation confirms PR #187. Review and merge remain open."),
]
MOCK_NOTES = [
    ("DEFAULT VIEW", "A returning human should be able to name the blocker, the affected revision and the next evidence from this screen."),
    ("PRESS 2", "Dependency arrows answer a different question: which later operation is waiting for this gate?"),
    ("PRESS ENTER", "The original citation stays exact at its own revision. A current binding needs independent evidence."),
    ("TWO EVENTS BEHIND", "An interrupted caller cannot conclude the PR failed. The effect keeps its identity while a reader reconnects."),
    ("CONFIRMED EFFECT", "The receipt answers one question: did this PR open? The rest of the run has its own completion boundary."),
]

def draw_pages(c, base, para, finish, color):
    base("RUN WATCH  /  HUMAN HANDOFF", "From a chat to a running change",
         "A synthetic rehearsal on a configured WSL host. The first message comes directly after the durable start receipt.")
    c.setFillColor(color("navy")); c.roundRect(40, 365, 1040, 214, 12, fill=1, stroke=0)
    para("TAREK", 63, 556, 910, 12, bold=True, fill="mint")
    para("Track the pagination change across API, SDK and CLI through opening the reviewed PRs.", 63, 529, 971, 19, 24, fill="pale")
    para("AGENT   /   AFTER RECEIPT", 63, 474, 910, 12, bold=True, fill="mint")
    para("Run demo/RUN-017. Open it in a second WSL terminal. I’ll stop at opened, reviewed PRs.", 63, 449, 971, 17, 22, fill="pale")
    c.setFont("Mono", 20); c.setFillColor(color("mint")); c.drawString(64, 391, "writfold run watch demo/RUN-017")
    for x, title, body in [
        (40,"Live view","Gates opens on the current blocker. Press 2 for Pipeline, Enter for evidence and q to detach. Work continues in the driver session."),
        (398,"One snapshot","Use writfold run show demo/RUN-017. Add --at 34 to return to the exact event someone reviewed."),
        (754,"Back next session","Ask for the run ID again. The agent reads the saved binding and hands you the same command.")]:
        para(title,x,321,315,19,bold=True,fill="teal"); para(body,x,274,314,15,22,fill="ink")
    para("The ref names the same run from any worktree in the configured realm. Access is checked separately.",40,112,1005,14,20,fill="muted")
    finish()
    base("RUN WATCH  /  AGENT PRACTICE", "One driver for a logical change",
         "A chat can contain several tasks. Worktrees, reconnects and new chats can all belong to the same run.")
    steps=[
        ("01","Asked for an ID","Read the bound run. If two candidates fit, ask which one. If nothing is tracked, say so."),
        ("02","Starting a tracked change","Persist the task key and intake. Return the coordinator’s committed ref immediately. Recover the same key after a lost reply."),
        ("03","Taking over a session","Reload the pinned procedure and exact run. Check repo revisions, owner generation and unresolved effects before work resumes."),
        ("04","Reporting progress","Give the cut, the observed blocker or finish line, and the next evidence. A watch command opens a reader."),
    ]
    for i,(num,title,body) in enumerate(steps):
        y=572-i*99
        para(num,40,y,53,15,bold=True,fill="teal");para(title,99,y+2,336,18,25,bold=True)
        para(body,463,y+1,604,15,21)
        c.setStrokeColor(color("line"));c.line(40,y-72,1080,y-72)
    para("Implement the text snapshot and receipt first. Turn on the session driver and native view only after the relevant proof, runtime and protocol gates pass.",40,143,1015,15,22,fill="muted")
    finish()
    for i,((title,sub),(label,caption)) in enumerate(zip(SCREENS,MOCK_NOTES),1):
        base(f"APPROVED SCREEN  /  {i:02} OF 05",title,sub)
        para(label,40,127,1000,12,bold=True,fill="teal")
        para(caption,40,99,1017,14,20)
        finish()

def overlay(pdf):
    """Reuse the approved vector terminal exactly. Keep the atlas framing and page number."""
    screen_path = ROOT / "examples/run-watch/approved-screens.pdf"
    with pymupdf.open(screen_path) as mocks, pymupdf.open(pdf) as atlas:
        assert len(mocks)==len(SCREENS) and len(atlas)>=63
        for i in range(len(mocks)):
            atlas[58+i].show_pdf_page(
                pymupdf.Rect(40,201,1080,650),mocks,i,
                clip=pymupdf.Rect(33,146,1047,618),keep_proportion=True)
        tmp=pdf.with_suffix(".pending.pdf")
        atlas.save(tmp,garbage=4,deflate=True,no_new_id=True,use_objstms=0)
    tmp.replace(pdf)
