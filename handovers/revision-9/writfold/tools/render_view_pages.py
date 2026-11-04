"""Five original atlas pages for the view and storage decision extension."""
from reportlab.lib.colors import HexColor


def draw_pages(c, base, para, finish, color):
    def card(x, y, w, h, title, body, fill='white'):
        c.setFillColor(HexColor('#FFFFFF') if fill == 'white' else color(fill))
        c.setStrokeColor(color('line'))
        c.roundRect(x, y, w, h, 12, fill=1, stroke=1)
        para(title, x+20, y+h-20, w-40, 17, 23, bold=True)
        para(body, x+20, y+h-60, w-40, 13, 20, fill='muted')

    def arrow(x, y, u, v):
        c.setStrokeColor(color('teal')); c.setLineWidth(2)
        c.line(x, y, u, v)
        c.setFillColor(color('teal'))
        p=c.beginPath()
        if x==u:
            sign=1 if v>y else -1
            p.moveTo(u,v);p.lineTo(u-5,v-sign*8);p.lineTo(u+5,v-sign*8)
        else:
            sign=1 if u>x else -1
            p.moveTo(u,v);p.lineTo(u-sign*8,v-5);p.lineTo(u-sign*8,v+5)
        p.close();c.drawPath(p,fill=1,stroke=0)

    base('VIEWS / ARCHITECTURE','One knowledge model. More ways to read it.',
         'Obsidian is the first human view channel. Future HTML can consume the same prepared document.')
    card(40,382,305,181,'Pinned knowledge','Release + context + policy + source closure. Filter the authorized audience before rendering.')
    card(407,382,305,181,'Prepared document','Typed headings, facts, evidence and notices. Assessment clock and coverage stay attached.')
    card(775,382,305,181,'Pure renderers','Portable Markdown and Obsidian today. HTML later; no parsing Markdown back into authority.')
    arrow(345,472,407,472);arrow(712,472,775,472)
    card(40,145,305,169,'Scribes acquire','Supervised tools return retained evidence. They cannot publish claims or write the view vault.')
    card(407,145,305,169,'Publisher owns effects','Validate paths and output closure. Install an immutable generation; activate conditionally.')
    card(775,145,305,169,'Obsidian reads','Bases, Dataview and the CLI navigate a snapshot. Editing a note is not knowledge admission.')
    arrow(927,382,927,345);arrow(927,345,560,345);arrow(560,345,560,314)
    arrow(712,228,775,228)
    para('New proof boundary: V14 must verify faithful projection; M10 specifies generation publication and process lifetime. The app and plugins remain explicit tested boundaries.',45,105,1030,13,19)
    finish()

    base('VIEWS / READING DESIGN','A page for a person, with evidence one click deeper.',
         'Design preview using synthetic content. This is not a screenshot or a native Obsidian qualification result.')
    c.setFillColor(HexColor('#FFFFFF'));c.roundRect(40,98,1040,479,12,fill=1,stroke=0)
    c.setFillColor(color('navy'));c.roundRect(40,98,230,479,12,fill=1,stroke=0)
    para('WRITFOLD',61,550,188,13,bold=True,fill='mint')
    para('Operations',61,510,188,19,bold=True,fill='pale')
    for y,title in [(460,'Overview'),(419,'Assets and attention'),(362,'Checkout'),(325,'Inventory'),(268,'Evidence'),(225,'Decisions'),(168,'My reading notes')]:
        if title=='Checkout':
            c.setFillColor(HexColor('#29464C'));c.roundRect(53,y-27,200,35,5,fill=1,stroke=0)
        para(title,65,y,180,12,18,fill='mint' if title=='Checkout' else 'pale')
    para('DEMO SNAPSHOT / OPERATIONS / 2 NOV 2026',302,551,720,10,15,fill='muted')
    para('Checkout',302,516,720,28,34,bold=True)
    para('Accepts a purchase request and asks Inventory to reserve the items. This design relationship is supported; runtime health is outside this export.',302,466,731,14,21)
    c.setFillColor(color('amberbg'));c.roundRect(302,321,738,79,8,fill=1,stroke=0)
    para('!  Owner needs evidence',319,384,700,14,bold=True,fill='amber')
    para('No current owner record is included. Missing evidence does not mean there is no owner.',319,356,700,12,18)
    para('What supports this page',302,296,730,18,bold=True)
    c.setStrokeColor(color('line'));c.line(302,254,1040,254)
    for x,t in [(308,'STATEMENT'),(630,'SUPPORT'),(851,'SCOPE')]: para(t,x,247,300 if x==308 else 180,9,bold=True,fill='muted')
    for y,a,b,d in [(216,'Depends on Inventory','Contract evidence','Design relationship'),(181,'Reservation example passed','Test receipt example','One synthetic fixture')]:
        para(a,308,y,307,11,16);para(b,630,y,208,11,16,fill='teal');para(d,851,y,180,11,16)
    para('›  Evidence details     ·     Nearby: Inventory     ·     Add a personal note',308,133,720,11,16,fill='teal')
    para('A short explanation, an unhidden gap, a small support table and explicit scope. Long provenance folds away; the only warning never does.',44,79,1030,12,17,fill='muted')
    finish()

    base('VIEWS / PLUGIN PROFILE','Tables that help you find the next useful question.',
         'Core Bases for everyday browsing; pinned Dataview for curated relational dashboards.')
    card(40,301,500,271,'Assets · core Bases','Table and card layouts share the same generation/context filters. No unscoped whole-vault query.')
    card(580,301,500,271,'Needs attention · Dataview','Fixed DQL templates select assessed labels. No generated JavaScript and no inference in the plugin.')
    for x,rows in [(60,[('Checkout','Needs evidence'),('Inventory','Supported')]),(600,[('Checkout','Owner unknown')])]:
        for i,(a,b) in enumerate(rows):
            yy=407-i*39;c.setFillColor(color('pale'));c.roundRect(x,yy-32,460,34,4,fill=1,stroke=0)
            para(a,x+10,yy-7,210,12,17,bold=True);para(b,x+240,yy-7,210,12,17,fill='muted')
    para('Synthetic snapshot · 2 services · no live health claim',60,330,460,10,15,fill='muted')
    para('Synthetic snapshot · capped at 25 rows',600,330,460,10,15,fill='muted')
    card(40,124,320,144,'Pin the actual bytes','Selected candidate: Dataview 0.5.68. The newer tag has a mismatched manifest; retain both observations.')
    card(400,124,320,144,'Install with intent','Exact vault and admitted plugin ID. CLI install has no version flag: verify assets before enabling.')
    card(760,124,320,144,'Keep it readable','Static notes and tables survive missing plugins. Outline, previews and backlinks help navigation.')
    para('WF23: real app/CLI versions, query results, indexing lag, light/dark/narrow reading and plugin-off fallback. These native checks remain to run on the PC.',45,90,1030,12,18,fill='muted')
    finish()

    base('VIEWS / REFRESH PROTOCOL','New snapshot first. Navigation changes last.',
         'Immutable generations preserve meaning when an old note remains open or the app indexes slowly.')
    for x,title,body in [(40,'Prepare','Pin source, context, grant and renderer. Create an immutable document.'),(407,'Validate','Escape text; check paths, links, closure and deterministic hashes.'),(775,'Install','Complete the new generation. Existing pages still refer to the old one.')]:
        card(x,384,305,178,title,body)
    arrow(345,471,407,471);arrow(712,471,775,471)
    for x,title,body in [(40,'Recovery','Interrupted preparation leaves the old Home intact. Preserve edited and human-owned files.'),(407,'Activate','Conditionally replace Home after closure. Reject a stale source or previous-index stamp.'),(775,'Observe','Wait for the reader or report indexing pending. Never promise atomic GUI refresh.')]:
        card(x,148,305,178,title,body)
    arrow(927,384,927,351);arrow(927,351,560,351);arrow(560,351,560,326)
    arrow(712,235,775,235);arrow(407,235,345,235)
    para('M10 must prove modeled closure/stamp/ownership/lifetime safety. Native crash and watcher tests qualify filesystem/app assumptions. Cleanup is separate and respects retained references.',45,106,1030,13,19)
    finish()

    from render_storage_pages import draw_pages as storage_pages
    storage_pages(c, base, para, finish, color)
