#!/usr/bin/env python3
"""Render the handover atlas and exact vector/PNG assets (requires ReportLab + Poppler)."""
from pathlib import Path
import json, math, html, subprocess, textwrap, re
from reportlab.pdfgen import canvas
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.lib.colors import HexColor, Color
from reportlab.lib.styles import ParagraphStyle
from reportlab.platypus import Paragraph
from reportlab.graphics.shapes import Drawing, Rect, Line, String, Polygon, PolyLine
from reportlab.graphics import renderPDF, renderSVG
ROOT=Path(__file__).resolve().parents[1]
TMP=ROOT.parent/'render-temporary'; TMP.mkdir(exist_ok=True)
for n,f in [('Sans','DejaVuSans.ttf'),('Bold','DejaVuSans-Bold.ttf'),('Mono','DejaVuSansMono.ttf')]:
    pdfmetrics.registerFont(TTFont(n,str(ROOT/'tools/fonts'/f)))
C={'ink':'#172D38','muted':'#536772','bg':'#F7F8F4','line':'#D9E2DF','teal':'#087D72','mint':'#6DD7C4','navy':'#101922','pale':'#EEF3F4','amber':'#9B5A18','amberbg':'#FFF0DA'}
def color(k):return HexColor(C.get(k,k))
def textlines(text,font,size,width):
    lines=[]
    for para in str(text).split('\n'):
        line=''
        for word in para.split():
            candidate=(line+' '+word).strip()
            if line and pdfmetrics.stringWidth(candidate,font,size)>width:lines.append(line);line=word
            else:line=candidate
        lines.append(line)
    return lines

def icon_svg(main,accent,bg=None):
    tile=f'<rect width="96" height="96" rx="22" fill="{bg}"/>' if bg else ''
    return f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96" role="img" aria-labelledby="title desc"><title id="title">Writfold</title><desc id="desc">A geometric W with a folded upper-right edge.</desc>{tile}<path fill="{main}" d="M14 22H26L37 56L48 38L59 56L70 22H82L64 76H56L48 62L40 76H32Z"/><path fill="{accent}" d="M70 22H82L76.7 38H64.7Z"/><path fill="{main}" fill-opacity=".18" d="M64.7 38H76.7L59 56Z"/></svg>'''
for name,m,a,b in [('writfold-icon.svg',C['ink'],C['teal'],None),('writfold-icon-dark.svg',C['pale'],C['mint'],None),('writfold-icon-mono.svg','currentColor','currentColor',None),('favicon.svg',C['pale'],C['mint'],C['navy'])]:
    (ROOT/'brand'/name).write_text(icon_svg(m,a,b)+'\n')

def draw_icon(c,x,y,size,main='ink',accent='teal',tile=None):
    c.saveState();c.translate(x,y);c.scale(size/96,size/96)
    if tile:c.setFillColor(color(tile));c.roundRect(0,0,96,96,22,fill=1,stroke=0)
    # SVG has downward y; invert to preserve exact shape.
    c.translate(0,96);c.scale(1,-1)
    def poly(points,fill):
        p=c.beginPath();p.moveTo(*points[0])
        for q in points[1:]:p.lineTo(*q)
        p.close();c.setFillColor(color(fill));c.drawPath(p,fill=1,stroke=0)
    poly([(14,22),(26,22),(37,56),(48,38),(59,56),(70,22),(82,22),(64,76),(56,76),(48,62),(40,76),(32,76)],main)
    poly([(70,22),(82,22),(76.7,38),(64.7,38)],accent)
    c.setFillAlpha(.18);poly([(64.7,38),(76.7,38),(59,56)],main)
    c.restoreState()

def diagram(s,header=True):
    d=Drawing(1200,610)
    if header: d.add(Rect(0,0,1200,610,fillColor=color('bg'),strokeColor=None))
    if header:
        d.add(String(48,566,s['id'],fontName='Bold',fontSize=19,fillColor=color('teal')))
        d.add(String(126,565,s['title'],fontName='Bold',fontSize=25,fillColor=color('ink')))
        for i,line in enumerate(textlines(s['goal'],'Sans',16,1104)):
            d.add(String(48,530-i*21,line,fontName='Sans',fontSize=16,fillColor=color('muted')))
    boxes=[(48,322,330,130),(435,322,330,130),(822,322,330,130),(48,80,330,130),(435,80,330,130),(822,80,330,130)]
    for ei,(a,b,label) in enumerate(s['edges']):
        x,y,w,h=boxes[a];u,v,bw,bh=boxes[b]
        if y==v:
            if abs(u-x)>w+100:
                mid=y+h+38+(ei%2)*18
                pts=[(x+w/2,y+h),(x+w/2,mid),(u+bw/2,mid),(u+bw/2,v+bh)]
            elif u>x:pts=[(x+w,y+h/2),(u,v+bh/2)]
            else:pts=[(x,y+h/2),(u+bw,v+bh/2)]
        else:
            if y>v:start=(x+w/2,y);end=(u+bw/2,v+bh)
            else:start=(x+w/2,y+h);end=(u+bw/2,v)
            if x == u and any(c == b and e == a for c,e,_ in s['edges']):
                offset = -88 if y > v else 88
                start = (start[0]+offset, start[1])
                end = (end[0]+offset, end[1])
            mid=242+(ei%3)*17
            pts=[start,(start[0],mid),(end[0],mid),end]
        d.add(PolyLine([z for pt in pts for z in pt],strokeColor=color('teal'),strokeWidth=2,fillColor=None))
        a1,a2=pts[-2],pts[-1];angle=math.atan2(a2[1]-a1[1],a2[0]-a1[0]);L=9
        tri=[a2,(a2[0]-L*math.cos(angle-.4),a2[1]-L*math.sin(angle-.4)),(a2[0]-L*math.cos(angle+.4),a2[1]-L*math.sin(angle+.4))]
        d.add(Polygon([z for pt in tri for z in pt],fillColor=color('teal'),strokeColor=None))
        if label:
            if len(pts)==2:lx=(pts[0][0]+pts[1][0])/2;ly=pts[0][1]+14
            else:lx=(pts[1][0]+pts[2][0])/2;ly=pts[1][1]+7
            # Keep labels off nodes; small horizontal arrows carry only short labels.
            tw=pdfmetrics.stringWidth(label,'Sans',11)
            if len(pts)==2 and tw>abs(pts[1][0]-pts[0][0])-8: continue
            d.add(Rect(lx-tw/2-4,ly-3,tw+8,15,fillColor=color('bg'),strokeColor=None))
            d.add(String(lx,ly,label,textAnchor='middle',fontName='Sans',fontSize=11,fillColor=color('teal')))
    for i,(box,content) in enumerate(zip(boxes,s['nodes'])):
        x,y,w,h=box;title,sub=content.split('|',1)
        d.add(Rect(x,y,w,h,rx=13,ry=13,fillColor=HexColor('#FFFFFF'),strokeColor=color('line'),strokeWidth=1.3))
        d.add(Rect(x,y+h-7,w,7,fillColor=color('teal') if i in [1,4] else color('line'),strokeColor=None))
        yy=y+h-32
        for line in textlines(title,'Bold',18,w-36):
            d.add(String(x+18,yy,line,fontName='Bold',fontSize=18,fillColor=color('ink')));yy-=23
        yy-=10
        for line in textlines(sub,'Sans',14,w-36):
            d.add(String(x+18,yy,line,fontName='Sans',fontSize=14,fillColor=color('muted')));yy-=19
        assert yy>y-2,(s['id'],title,yy,y)
    return d
stages=json.loads((ROOT/'registries/stages.json').read_text())
overview={'id':'SYSTEM','title':'One engine. Explicit evidence.','goal':'Scribes acquire; the kernel assesses; the release protocol publishes.','nodes':['Agent or human|CLI and initialized JSON-RPC','Writfold application|Typed services + verified core','Scribes|Admitted external scripts','Git|Authored files + release manifests','Omnigraph|Graph + commit + schema epoch','Writs|Retained source bytes + provenance'],'edges':[(0,1,''),(1,2,'acquire'),(1,3,'publish'),(1,4,'pin'),(2,5,'validate + retain'),(5,4,'propose and approve')]}
for s in [overview]+stages:
    d=diagram(s);svg_path=ROOT/'diagrams'/f"{s['id']}.svg"
    renderSVG.drawToFile(d,str(svg_path))
    svg=re.sub(r'<!DOCTYPE[^>]*>', '', svg_path.read_text(), flags=re.S)
    svg=svg.replace('<title>...</title>', '<title>'+html.escape(s['title'])+'</title>')
    svg=svg.replace('<desc>...</desc>', '<desc>'+html.escape(s['goal'])+'</desc>')
    svg_path.write_text(svg)
    tmp=TMP/f"{s['id']}.pdf";renderPDF.drawToFile(d,str(tmp))
    subprocess.run(['pdftoppm','-png','-singlefile','-scale-to-x','1440','-scale-to-y','-1',str(tmp),str(ROOT/'diagrams'/s['id'])],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
# Icons exact vector-to-raster via PDF.
for name,main,accent,tile in [('writfold-icon','ink','teal',None),('writfold-icon-dark','pale','mint',None),('favicon','pale','mint','navy')]:
    path=TMP/(name+'.pdf');c=canvas.Canvas(str(path),pagesize=(256,256));
    if tile is None:c.setFillColor(color('bg' if main=='ink' else 'navy'));c.rect(0,0,256,256,fill=1,stroke=0)
    draw_icon(c,0,0,256,main,accent,tile);c.save()
    subprocess.run(['pdftoppm','-png','-singlefile','-scale-to','512',str(path),str(ROOT/'brand'/name)],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)

W,H=1120,790
pdf=ROOT/'review-atlas.pdf';c=canvas.Canvas(str(pdf),pagesize=(W,H),invariant=1);c.setTitle('Writfold — Rust implementation review atlas');c.setAuthor('Prepared for Tarek Nawara')
page=0

def para(text,x,y,w,size=14,leading=None,fill='ink',bold=False):
    style=ParagraphStyle('p',fontName='Bold' if bold else 'Sans',fontSize=size,leading=leading or size*1.4,textColor=color(fill))
    p=Paragraph(html.escape(text),style);_,hh=p.wrap(w,1000);p.drawOn(c,x,y-hh);return y-hh

def base(label,title,subtitle):
    global page
    page+=1;c.setFillColor(color('bg'));c.rect(0,0,W,H,fill=1,stroke=0)
    draw_icon(c,38,H-65,34)
    c.setFillColor(color('ink'));c.setFont('Bold',14);c.drawString(80,H-51,'writfold')
    c.setFont('Sans',11);c.setFillColor(color('muted'));c.drawRightString(W-40,H-51,label)
    c.setStrokeColor(color('line'));c.line(40,H-82,W-40,H-82)
    para(title,40,H-110,W-80,29,35,bold=True)
    para(subtitle,40,H-160,W-80,14,20,fill='muted')
    c.setStrokeColor(color('line'));c.line(40,40,W-40,40)
    c.setFont('Sans',10);c.setFillColor(color('muted'));c.drawString(40,23,'4 November 2026 · Revision 9 · Implementation and proof plan')
    c.drawRightString(W-40,23,f'{page:02} / 68')

def finish():c.showPage()

base('REVIEW ATLAS / REVISION 9','Writfold implementation review','Citation proofs: pages 2-7. Run Watch: 57-63. Incident pilot: 64-66. General discovery and Metamath: 67-68.')
draw_icon(c,55,382,215,'pale','mint','navy')
y=572
for line in ['First   /   Prove citation binding','Next   /   Qualify the compact reader','Then   /   Enable proven proposal paths','Later   /   Evaluate the new views']:
    y=para(line,315,y,720,23,31,bold=True)-12
c.setFillColor(color('amberbg'));c.roundRect(40,160,1040,150,14,fill=1,stroke=0)
para('What opens proposal admission',62,285,980,18,bold=True,fill='amber')
para('Prove the rule in Lean, then kernel-check its application to the exact proposal, witness and predecessor. Required production, protocol and authority checks must also pass.',62,250,975,15,22)
para('Release inventory: 27 stages, 34 gates, 23 Verus obligations, 16 protocol models and 164 scenarios. E01-E12 and CIT01-CIT08 are proposed Lean targets. New proof receipts are pending.',40,125,1030,13,19,fill='muted')
finish()
from render_citation_pages import draw_pages as citation_pages
citation_pages(c, base, para, finish, color)
base('ARCHITECTURE','Small core. Explicit boundaries.','The catalog routes capabilities; external scribes acquire evidence without authority to publish claims.')
d=diagram(overview,False);d.scale(.86,.86);renderPDF.draw(d,c,44,180)
para('The completed Git release pins the exact Omnigraph graph/commit/epoch pin, signed operation and retained evidence closure. Prepared objects and recovery journals are not publication authority.',55,162,1010,15,22)
para('Runtime: Linux/WSL x86_64. Native Rust application + managed Omnigraph + explicit Git dependency. Proof tools and Python remain outside the core runtime.',55,94,1010,12,18,fill='muted')
finish()
base('ASSURANCE','Prove the contract you actually ship.','Keep mathematical semantics, executable verification, protocol safety and real-process evidence separate.')
rows=[('LEAN','Language and judgments','353 baseline declarations in T01-T22. Engineering E01-E12 and citation CIT01-CIT08 carry separate pending proof obligations.'),('VERUS','Actual executable decisions','V01-V23 cover knowledge, engineering, citations and admission. Production callers establish preconditions.'),('TLA+ / TLC','Interleavings and failures','M01-M16; knowledge, ledger, effects, skills, learning and projection models. Include environment changes and calibrated negative cases.'),('TLAPS','Inductive safety proofs','Init, every action, frame/stutter cases and cross-model bridges. Temporal claims stay within executed tool support.')]
y=558
for tag,title,desc in rows:
    c.setFillColor(HexColor('#FFFFFF'));c.roundRect(40,y-92,1040,92,10,fill=1,stroke=0)
    para(tag,60,y-19,170,15,bold=True,fill='teal');para(title,240,y-14,810,16,bold=True);para(desc,240,y-42,810,12,18,fill='muted');y-=108
para('Trusted boundaries remain visible: source truth, cryptographic libraries, OS durability, Git/Omnigraph semantics, sandbox enforcement and cross-prover correspondence. Tests support those boundaries; they do not turn them into proofs.',45,110,1030,13,19)
finish()
base('WORKFLOW 3','More contexts. The same inference boundaries.','Portfolios and design binding add explicit authority without changing the core inference rules.')
for xx,tag,title,body in [(40,'PORTFOLIOS','One release, separate assessments','A complete signed family of named contexts. Each new claim has an exact context and witness assignment. No source, receipt or policy borrowing across contexts.'),(580,'DESIGN BINDING','Approve intent, then bind implementation','Accept a stable design before code exists. Bind an exact source and target later. Rebinding preserves acceptance, but old-target passes stay historical.')]:
    c.setFillColor(HexColor('#FFFFFF'));c.roundRect(xx,304,500,280,12,fill=1,stroke=0)
    para(tag,xx+22,560,455,12,bold=True,fill='teal');para(title,xx+22,522,455,23,30,bold=True);para(body,xx+22,432,455,16,24)
para('Historical authority is not upgraded by relabelling.',40,254,1040,22,bold=True)
para('Completed schema-2 releases keep their original bytes and signature domain. Pending old writes cannot resume through the schema-3 writer. Queries name a context; forward selection preserves current corrections and policy.',40,212,1040,16,24)
para('New stages: P03P portfolios and P08D design binding. New proofs: V12, V13 and M09. Full contract: plan/11.',40,100,1040,13,19,fill='muted')
finish()
base('PLAYBOOK ADOPTION','Use the patterns. Keep the concrete engine.','Pinned to private private:playbook at e7d991e; the new view channel follows I1/I2/I3 without a generic framework.')
rows=[('S1 / S2','Explicit decisions and checked stages','Production Rust predicates; exact prepared plans; callers establish preconditions.'),('S3 / I3','Controlled effects and recovery','One owner per field; admitted capabilities; durable closure before visibility.'),('I1','Prepared presentation','CLI and RPC render the same immutable engine view. Rendering performs no IO or mutation.'),('I2','Stamped results and owned lifetimes','Check session/request/source/grant identity. Cancellation holds capacity until physical teardown.')]
y=558
for tag,title,desc in rows:
    c.setFillColor(HexColor('#FFFFFF'));c.roundRect(40,y-92,1040,92,10,fill=1,stroke=0)
    para(tag,60,y-19,170,15,bold=True,fill='teal');para(title,240,y-14,810,16,bold=True);para(desc,240,y-42,810,12,18,fill='muted');y-=108
para('WF21 keeps the adoption record, production mapping, diagrams and this companion current. Updating Writfold does not require editing the entire upstream playbook. Private reference material is not published on the website.',45,110,1030,13,19)
finish()
base('WEBSITE OUTLINE','A minimal home for writfold.dev.','One useful page, inspired by Gripsack’s current design. Complete section copy and acceptance rules are in plan/10.')
# Scaled desktop wireframe; right rail explains exact sections.
x,y,w,h=44,90,640,480
c.setFillColor(color('navy'));c.roundRect(x,y,w,h,14,fill=1,stroke=0)
draw_icon(c,x+22,y+h-50,28,'pale','mint');c.setFont('Bold',12);c.setFillColor(color('pale'));c.drawString(x+60,y+h-40,'writfold')
c.setFont('Sans',9);c.setFillColor(color('mint'));c.drawRightString(x+w-24,y+h-39,'start    scribes    docs    GitHub')
para('Keep the context.\nKnow what supports it.',x+32,y+h-88,w-64,27,34,fill='pale',bold=True)
para('Bring evidence from your tools into one durable knowledge base. Keep its source, history and approval attached.',x+32,y+h-173,w-64,12,18,fill='pale')
c.setFillColor(color('mint'));c.roundRect(x+32,y+h-241,115,30,6,fill=1,stroke=0);c.setFillColor(color('ink'));c.setFont('Bold',10);c.drawString(x+47,y+h-230,'get started')
c.setFont('Sans',9);c.setFillColor(color('pale'));c.drawString(x+165,y+h-230,'Linux x86_64 · WSL2')
for i,(a,b) in enumerate([('Keep the evidence.','Exact source and history.'),('Review what changes.','Explicit approval.'),('Carry context forward.','Support and visible gaps.')]):
    xx=x+32+i*194;para(a,xx,y+197,180,11,16,fill='mint',bold=True);para(b,xx,y+170,180,9,14,fill='pale')
c.setStrokeColor(HexColor('#33454E'));c.line(x+32,y+135,x+w-32,y+135)
para('start with one source',x+32,y+112,260,15,bold=True,fill='pale')
para('One real workflow. Then connect a tool with a small scribe.',x+32,y+79,w-64,11,17,fill='pale')
c.setFont('Sans',9);c.setFillColor(color('mint'));c.drawString(x+32,y+27,'install / offline bundle     docs     assurance     source')
yy=555
for tag,desc in [('01 Hero','One clear promise. No fake install command.'),('02 Three capabilities','Evidence, reviewed changes, reusable context.'),('03 One workflow','Tested commands and a real synthetic result.'),('04 Scribes','Simple scripts; explicit evidence boundaries.'),('05 Install + docs','Exact release and honest prerequisites.'),('06 Footer','Source, changelog and confirmed license.')]:
    yy=para(tag,725,yy,340,16,bold=True)-6;yy=para(desc,725,yy,340,12,18,fill='muted')-20
finish()
base('BRAND ASSETS','A folded W, built as a real vector.','Light, dark, monochrome and favicon SVGs are included. The mark contains no embedded raster, scripts or external resources.')
for i,(label,bg,main,accent) in enumerate([('Light surface','bg','ink','teal'),('Dark surface','navy','pale','mint'),('Favicon','line','pale','mint')]):
    xx=40+i*360;c.setFillColor(color(bg));c.roundRect(xx,270,320,290,16,fill=1,stroke=0)
    draw_icon(c,xx+60,327,200,main,accent,'navy' if i==2 else None)
    para(label,xx+20,303,280,14,bold=True,fill='pale' if i==1 else 'ink')
para('The upper-right facet folds inward. Keep the silhouette consistent across themes; change only its defined colors. Use transparent SVG for UI/README and the tiled favicon for browser tabs.',45,224,1030,15,22)
para('Writfold is settled. Scribes are supervised evidence adapters; writs are retained evidence records. Historical kctx identifiers and signed bytes keep their original identity.',45,140,1030,14,21,fill='muted')
finish()
from render_view_pages import draw_pages
draw_pages(c, base, para, finish, color)
from render_engineering_pages import draw_pages as engineering_pages
engineering_pages(c, base, para, finish, color)
for s in stages:
    base(s['id']+' · '+', '.join(s['dependencies'] or ['start']),s['title'],s['goal'])
    d=diagram(s,False);d.scale(.86,.86);renderPDF.draw(d,c,44,175)
    para('BUILD',44,211,480,11,bold=True,fill='teal');para('EXIT GATE',590,211,490,11,bold=True,fill='teal')
    yy=187
    for text in s['deliverables'][:3]:yy=para('• '+text,44,yy,495,11.5,16)-7
    yy2=187
    for text in s['acceptance']:yy2=para('• '+text,590,yy2,490,11.5,16)-7
    assert min(yy,yy2)>47,(s['id'],yy,yy2)
    finish()
from render_run_watch import draw_pages as run_watch_pages, overlay
run_watch_pages(c, base, para, finish, color)
from render_discovery_pages import draw_pages as discovery_pages
discovery_pages(c, base, para, finish, color)
from render_general_find import draw_pages as general_find_pages
general_find_pages(c, base, para, finish, color)
c.save()
overlay(pdf)
print('Rendered',pdf,'pages=',page)
# Offline HTML navigator: no network dependencies, every stage diagram inline via file.
nav=''.join(f'<a href="#'+s['id']+'">'+s['id']+' '+html.escape(s['title'])+'</a>' for s in stages)
sections=''
for s in stages:
    sections+=f'<section id="{s["id"]}"><span class="tag">{s["id"]}</span><h2>{html.escape(s["title"])}</h2><p>{html.escape(s["goal"])}</p><img src="diagrams/{s["id"]}.svg" alt="{html.escape(s["title"])}"><div class="cols"><div><h3>Build</h3><ul>'+''.join('<li>'+html.escape(x)+'</li>' for x in s['deliverables'])+'</ul></div><div><h3>Exit gate</h3><ul>'+''.join('<li>'+html.escape(x)+'</li>' for x in s['acceptance'])+f'</ul></div></div><p><a href="{s["file"]}">Full stage instructions</a> · Gates: '+', '.join(s['gates'])+'</p></section>'
index='''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Writfold implementation handover</title><style>html{scroll-behavior:smooth}body{margin:0;background:#F7F8F4;color:#172D38;font:16px/1.6 system-ui}aside{position:fixed;width:240px;inset:0 auto 0 0;overflow:auto;padding:24px;background:#101922;color:#EEF3F4;box-sizing:border-box}aside a{display:block;color:#A8CFC8;text-decoration:none;font-size:13px;padding:6px 0}main{max-width:1100px;margin-left:240px;padding:42px}h1{font-size:48px;line-height:1.1}h2{font-size:28px}a{color:#087D72}section{border-top:1px solid #D9E2DF;margin-top:44px;padding-top:32px;scroll-margin-top:20px}img{max-width:100%}.tag{color:#087D72;font-weight:700}.cols{display:grid;grid-template-columns:1fr 1fr;gap:30px}.note{padding:20px;background:#FFF0DA;border-radius:10px}.brand{width:44px;vertical-align:middle}li{margin:7px 0}@media(max-width:800px){aside{position:static;width:auto}aside nav{display:none}main{margin:0;padding:24px}.cols{grid-template-columns:1fr}h1{font-size:36px}}@media(prefers-reduced-motion:reduce){html{scroll-behavior:auto}}</style><aside><img class="brand" src="brand/writfold-icon-dark.svg" alt="Writfold"><strong> writfold</strong><p>Implementation handover</p><nav>'''+nav+'''</nav></aside><main><span class="tag">RUST / LINUX / WSL</span><h1>Knowledge with a record.</h1><p>Revision 9. Read <a href="plan/30-corpus-discovery.md">General corpus discovery</a>, <a href="plan/28-similar-incident-discovery.md">Similar cases</a>, <a href="plan/29-omnigraph-capability-fit.md">Omnigraph capability review</a> and <a href="plan/27-run-watch-and-session-handoff.md">Run Watch and the session handoff</a> and the <a href="examples/run-watch/approved-screens.pdf">five approved screens</a>. Begin with the two citation stages. The full inventory contains 27 stages and 34 gates.</p><p><a href="PRIORITIES.md">Priority queue</a> · <a href="plan/24-citation-calculus.md">Citation calculus</a> · <a href="plan/25-proof-first-admission.md">Proof before admission</a> · <a href="plan/26-compact-citations-and-followups.md">Reader experiments</a> · <a href="AGENT_START_HERE.md">Agent start here</a> · <a href="review-atlas.pdf">Visual atlas PDF</a> · <a href="plan/01-review-findings.md">Repository review</a> · <a href="plan/09-scribes-and-writs.md">Scribes protocol</a> · <a href="plan/10-minimal-website-outline.md">Website outline</a> · <a href="plan/11-workflow3-contract.md">Workflow 3</a> · <a href="plan/12-playbook-adoption.md">Playbook adoption</a> · <a href="plan/13-view-channels.md">View channels</a> · <a href="plan/14-obsidian-reading-design.md">Obsidian design</a> · <a href="plan/15-storage-decision.md">Storage decision</a> · <a href="plan/16-omnigraph-storage-contract.md">Omnigraph contract</a> · <a href="plan/17-engineering-graphs-and-layout.md">Engineering layout</a> · <a href="plan/18-identities-fingerprints-and-cuts.md">Fingerprints</a> · <a href="plan/22-engineering-calculus.md">Proposed calculus</a> · <a href="plan/23-engineering-extension-acceptance.md">Extension acceptance</a> · <a href="examples/obsidian-vault/README.md">Sample vault</a></p><div class="note"><strong>Revision 9: general corpus discovery, proof-first citation work, Run Watch and scoped incident discovery.</strong> Core semantics remain v2; workflows use schema 3 and RPC protocol 2. The old graph warning defect is fixed. Read plans/17–23 for the two graph families, ledger, fingerprints, skill composition, cross-repo execution, learning and proposed calculus. Plans/15–16 retain the early P01O storage gate and immutable epochs. Native Obsidian and complete WSL qualification remain pending.</div>'''+sections+'''<section><h2>Source and integrity</h2><p>This is an implementation plan, not a completed Rust product. See README, the source archive and evidence directory. Run <code>python3 tools/check_bundle.py</code> to validate the bundle manifest.</p></section></main></html>'''
(ROOT/'index.html').write_text(index)
# Standalone website wireframe is a static visual outline, not deployed HTML.
(ROOT/'website/README.md').write_text('# Website outline\n\nSee `../plan/10-minimal-website-outline.md` for the six-section minimal design, and page 12 of `../review-atlas.pdf` for the visual wireframe. Brand assets are in `../brand/`. Implement the separate Pages repository only after the product and account prerequisites are satisfied.\n')
print('Assets and offline navigator complete')
