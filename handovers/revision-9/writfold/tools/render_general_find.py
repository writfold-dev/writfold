"""Two human-facing pages for general discovery and the supplementary proof."""
def draw_pages(c, base, para, finish, color):
    def row(y, label, title, detail):
        para(label,40,y,145,12,17,fill='teal',bold=True)
        para(title,200,y+1,315,17,23,bold=True)
        para(detail,550,y+1,530,15,20)
        c.setStrokeColor(color('line')); c.line(40,y-65,1080,y-65)
    base('CORPUS DISCOVERY  /  PRODUCT SCOPE','Search the whole knowledge base',
         'One find call can bring back several kinds of evidence. Each hit keeps its own revision, source and role.')
    c.setFillColor(color('navy'));c.roundRect(40,468,1040,100,12,fill=1,stroke=0)
    para('ONE QUESTION',62,550,950,12,18,fill='mint',bold=True)
    para('“How have we safely changed a response schema across repos?”',62,514,975,20,26,fill='pale',bold=True)
    row(440,'DESIGN','A coexistence decision','Open the accepted decision and check whether anything superseded it.')
    row(355,'REVIEW + CODE','A PR comment and a test','Open the diff and its disposition. Try the regression against the current decoder.')
    row(270,'PROCEDURE','An author-change skill','Inspect the pinned procedure and preconditions. Execution requires its own grant.')
    row(185,'INCIDENT','A previous client failure','Find the historical discriminator and check it in today’s telemetry.')
    para('Incidents are the first benchmark. Service dossiers and admitted lessons also belong in discovery.',40,96,1030,14,20,fill='teal',bold=True)
    finish()
    base('PROOF NOTES  /  METAMATH','What did the proof checker do?',
         'Metamath lets you write a claim as a sequence of explicit steps. A small verifier checks each step against the rules.')
    para('Two histories in the abstract model',40,560,980,20,26,bold=True)
    c.setFillColor(color('navy'));c.roundRect(40,385,1040,121,12,fill=1,stroke=0)
    for tag,txt,y in [('A','Read design  →  read review  →  submit checked proposal P',468),
                      ('B','Submit the same checked proposal P',421)]:
        para(tag,62,y,38,15,20,fill='mint',bold=True)
        para(txt,120,y,905,18,24,fill='pale')
    para('Why the outcomes agree',40,351,460,18,24,fill='teal',bold=True)
    para('Read steps preserve admitted state. For the same starting state and the same sequence of checked submissions, induction covers any finite number of intervening reads.',40,308,470,15,23)
    para('What the agent can still change',563,351,490,18,24,fill='teal',bold=True)
    para('Reading the review may persuade the agent to submit a different proposal. That changes the inputs to the comparison. No theorem here says a hit is relevant or that its historical fix applies today.',563,308,492,15,23)
    para('Metamath supplied an independent check. The project’s Lean and Rust qualification gates remain open.',40,117,1010,15,22,fill='teal',bold=True)
    finish()
