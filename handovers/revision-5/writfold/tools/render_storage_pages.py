"""Backend decision and historical-epoch pages for the review atlas."""
from reportlab.lib.colors import HexColor


def draw_pages(c, base, para, finish, color):
    def box(x, y, w, h, title, body, accent=False):
        c.setFillColor(color('pale') if accent else HexColor('#FFFFFF'))
        c.setStrokeColor(color('line'))
        c.roundRect(x, y, w, h, 12, fill=1, stroke=1)
        para(title, x+18, y+h-18, w-36, 17, 23, bold=True)
        para(body, x+18, y+h-59, w-36, 13, 19, fill='muted')

    def arrow(points):
        c.setStrokeColor(color('teal'));c.setLineWidth(2)
        p=c.beginPath();p.moveTo(*points[0])
        for xy in points[1:]:p.lineTo(*xy)
        c.drawPath(p)
        x,y=points[-2];u,v=points[-1];p=c.beginPath();p.moveTo(u,v)
        if x==u:
            d=1 if v>y else -1;p.lineTo(u-5,v-8*d);p.lineTo(u+5,v-8*d)
        else:
            d=1 if u>x else -1;p.lineTo(u-8*d,v-5);p.lineTo(u-8*d,v+5)
        p.close();c.setFillColor(color('teal'));c.drawPath(p,fill=1,stroke=0)

    base('STORAGE / APPROVED DIRECTION','Omnigraph first. Qualify before integration.',
         'One intended primary backend; an early blocking gate before Writfold relies on its storage contract.')
    box(40,397,320,174,'Primary target','Omnigraph stores structured knowledge and typed relationships. Exact judgments and discovery have distinct contracts.',True)
    box(400,397,320,174,'Legacy boundary','Dolt remains reference/import tooling and an explicit fallback decision. No second normal writer or automatic retry path.')
    box(760,397,320,174,'Early gate: P01O','WF24 qualifies native semantics, epochs, restore, ownership and workload budgets on the actual WSL profile.')
    para('What the experiments actually established',44,365,1030,19,bold=True)
    rows=[('Change an existing type','Old whole-node query refuses after a field addition.'),
          ('Preserve V1; introduce V2','Old whole-node rows remain identical in the follow-up.'),
          ('Branch from the older schema','Refused. Reading history is a different operation.'),
          ('Production admission','Not established: the full foundation and integrated gates remain to run.')]
    for i,(a,b) in enumerate(rows):
        y=321-i*45;c.setStrokeColor(color('line'));c.line(44,y-29,1076,y-29)
        para(a,50,y,330,12,18,bold=True);para(b,404,y,660,12,18)
    para('Keep the calculus and signed release boundary. Change the storage adapter, provisioned closure, migration protocol and proof assumptions explicitly.',44,111,1030,14,21)
    para('Retained evidence: original comparison 12 assertions / 19 commands; follow-up 6 assertions / 7 commands. Neither is WSL/server/crash qualification.',44,67,1030,10.5,15,fill='muted')
    finish()

    base('STORAGE / HISTORICAL MEANING','A new schema gets a new epoch.',
         'Old releases keep their graph, commit, schema and query contract. A migration prepares a separate successor.')
    box(40,389,320,180,'Epoch 1: retained','Immutable physical schema. Old release pins its graph commit and versioned historical queries.')
    box(400,389,320,180,'Approved mapping','Pin the source; account for every record. Preserve IDs, contexts, corrections and evidence.')
    box(760,389,320,180,'Epoch 2: candidate','Initialize a separate graph. Reconstruct canonical state and verify the complete migration.')
    arrow([(360,478),(400,478)]);arrow([(720,478),(760,478)])
    box(40,143,320,177,'Old readers continue','Existing pins remain valid. Retain the old native roots, query bytes and compatible runtime profile.')
    box(400,143,320,177,'Root, validate, CAS','Retain the complete candidate. Recheck source and predecessor; publish via the completed Git ref.',True)
    box(760,143,320,177,'New readers select','Only the completed release selects epoch 2. Retire epoch 1 after every retention obligation ends.')
    arrow([(200,389),(200,320)])
    arrow([(920,389),(920,355),(560,355),(560,320)])
    arrow([(720,230),(760,230)])
    para('No cross-schema branch shortcut. New proposals target the current release; genuine historical-schema replay uses an isolated verified restoration.',44,110,1030,13,19)
    para('Required: V15 storage-pin/migration admission + M11 epoch safety, with bridges to publication, recovery, retention and views. These are proof obligations, not completed proofs.',44,66,1030,10.5,15,fill='muted')
    finish()
