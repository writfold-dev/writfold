"""Ten precise vector atlas pages for the revision-5 engineering extension."""
from reportlab.lib.colors import HexColor


def draw_pages(c, base, para, finish, color):
    def box(x, y, w, h, title, body, accent=False, size=13):
        c.setFillColor(color('pale') if accent else HexColor('#FFFFFF'))
        c.setStrokeColor(color('line'))
        c.roundRect(x, y, w, h, 12, fill=1, stroke=1)
        yy=para(title,x+18,y+h-17,w-36,17,22,bold=True)
        end=para(body,x+18,yy-12,w-36,size,size*1.45,fill='muted')
        assert end>=y+10,(title,end,y)

    def arrow(points, label=None):
        c.setStrokeColor(color('teal'));c.setLineWidth(2)
        path=c.beginPath();path.moveTo(*points[0])
        for xy in points[1:]:path.lineTo(*xy)
        c.drawPath(path)
        x,y=points[-2];u,v=points[-1]
        p=c.beginPath();p.moveTo(u,v)
        if x==u:
            sign=1 if v>y else -1;p.lineTo(u-5,v-sign*8);p.lineTo(u+5,v-sign*8)
        else:
            sign=1 if u>x else -1;p.lineTo(u-sign*8,v-5);p.lineTo(u-sign*8,v+5)
        p.close();c.setFillColor(color('teal'));c.drawPath(p,fill=1,stroke=0)
        if label:
            para(label,points[0][0]+8,points[0][1]-4,260,10,14,fill='teal')

    def rows(items, y=558, h=67, left=300):
        for a,b in items:
            c.setFillColor(HexColor('#FFFFFF'));c.roundRect(40,y-h+5,1040,h-8,8,fill=1,stroke=0)
            aa=para(a,57,y-10,left-27,12.5,18,bold=True)
            bb=para(b,57+left,y-10,1007-left,12.5,18,fill='muted')
            assert min(aa,bb)>y-h+9,(a,aa,bb,y-h)
            y-=h
        return y

    base('ENGINEERING / STORAGE MAP','Three semantic graphs. Two storage families.',
         'One managed Omnigraph companion per trust realm. Each family retains independent schema epochs.')
    box(40,394,320,175,'Context','Admitted claims, witnesses, sources, corrections and selected procedure definitions.',True)
    box(400,394,320,175,'Execution','Procedure structure, actual runs, attempts, effects and checks. Definitions and instances stay distinct.')
    box(760,394,320,175,'Learning','Observations, candidates, evaluation and promotion into new admitted revisions.')
    box(40,158,500,165,'Corpus family','Admitted knowledge, skill/procedure revisions and accepted lessons. A completed Git release selects the exact native pin.',True)
    box(580,158,500,165,'Activity family','Queryable run and learning projections from the durable EngineeringLedger. Exact watermarks; never effect authority.')
    arrow([(200,394),(200,323)])
    arrow([(510,394),(510,357),(360,357),(360,323)])
    arrow([(610,394),(610,357),(720,357),(720,323)])
    arrow([(920,394),(920,323)])
    para('Learning promotion crosses through the existing admission protocol: retain evidence, validate the candidate, then publish the completed Git release.',45,118,1030,14,21)
    para('Two logical families can mean more than two physical graphs while historical epochs remain retained. No cross-graph transaction is assumed.',45,69,1030,11.5,17,fill='muted')
    finish()

    base('ENGINEERING / AUTHORITY','Which record can justify the next action?',
         'Queryability, recorded history and current permission have different owners.')
    rows([
      ('Corpus release','Exact graph/commit/epoch/query pin plus signed retained closure. Preserves version-4 knowledge authority.'),
      ('EngineeringLedger','Immutable records and conditional stream heads select operational history. Reconstruct with the checked reducer.'),
      ('Current guards','Applicable grants, ownership, subject bindings and external preconditions are checked at effect admission.'),
      ('Activity graph','Derived projection with a watermark. A stale or forged status row cannot authorize a merge.'),
      ('Recovery hints','Describe interrupted preparation. They never substitute for validated history, approval or observed outcomes.'),
      ('Promotion evidence','Retained in corpus closure before publication, so activity pruning cannot erase a lesson\'s justification.')
    ],h=68)
    para('Local ledger origin relies on the qualified coordinator and private-filesystem boundary. Hash chains establish linkage; they do not authenticate arbitrary actors.',45,113,1030,13,19)
    finish()

    base('ENGINEERING / PHYSICAL LAYOUT','A realm has explicit ownership on disk.',
         'Authoritative state stays on qualified Linux storage. Worktrees and the external BuildKit setup keep their chosen locations.')
    rows([
      ('identity.json + releases.git/','Realm/KbId and local instance; retained Git release manifests and source objects.'),
      ('objects/sha256/','Exact source bytes, package resources, ledger records, receipts and admission evidence.'),
      ('ledger/heads/','Conditional stream-head records. Stable logical identity; exact immutable event prefix.'),
      ('coordination/','Cross-run resource reservations and owner generations; stable managed lock names.'),
      ('omnigraph/cluster/','Whole engine-owned root: corpus/activity epochs, native history and metadata.'),
      ('deployment/ + bindings/','Pinned schemas/queries/profiles; separate local roots, endpoints and worktree locators.'),
      ('Runtime, cache and views','Private process endpoints; disposable caches; separately owned Obsidian generations and Human notes.')
    ],h=60,left=310)
    para('Default realm root: $XDG_DATA_HOME/writfold/realms/<RealmId>/. Paths are locators, not portable identity. Native backup captures the complete qualified root.',45,111,1030,13,19)
    finish()

    base('ENGINEERING / FINGERPRINTS','Name the exact thing a check depends on.',
         'Logical IDs, native revisions, content digests and current authorization stamps remain distinct nominal types.')
    rows([
      ('Package + dependency lock','Exact resource bytes and modes; flat resolved dependency graph. Document cycles do not create recursive hashes.'),
      ('Skill + procedure revision','Entry/loader profile, package closure, typed control flow, contracts and required gates.'),
      ('Repo vector + read set','Actual code/input snapshots across repos; exact corpus contexts, witnesses, coverage and assessment inputs.'),
      ('Environment + attempt input','Tools, dependency locks, relevant configuration, fixtures and the exact step subject.'),
      ('Effect intent + dispatch stamp','Stable logical operation survives recovery; current grant/head/owner/incarnation gets a fresh checked stamp.'),
      ('Receipt + learning candidate','Exact result/evidence and proposed delta. Identity does not imply usefulness, approval or truth.'),
      ('Engineering read cut','Corpus pin + activity pin + stream/inventory watermarks, visibility, time and completeness.')
    ],h=60,left=300)
    para('Proposed encoding: restricted deterministic CBOR + domain-separated SHA-256. Matching fingerprints still require freshness, authority, scope and complete evidence.',45,110,1030,13,19)
    finish()

    base('ENGINEERING / SKILLS','Keep the Markdown. Check the composition.',
         'Existing Python and shell remain usable through admitted execution profiles; loading guidance does not grant effects.')
    box(40,389,320,180,'Entry and router','Select an admitted skill revision. Load only the relevant route and required context.',True)
    box(400,389,320,180,'References','Read exact pinned documents progressively. Follow document cycles once per resource revision.')
    box(760,389,320,180,'Nested procedure','Typed inputs, preconditions, inherited gates, bounded calls and exact return evidence.')
    arrow([(360,479),(400,479)]);arrow([(720,479),(760,479)])
    box(40,155,320,165,'Pinned package','All resources and executable dependencies retained. Edits create a new revision.')
    box(400,155,320,165,'Scoped activation','Run, inputs, repository scope, parent invocation and capability intersection.')
    box(760,155,320,165,'Effect adapter','Check current authority; supervise scripts and descendants; verify actual postconditions.')
    arrow([(200,389),(200,354),(560,354),(560,320)])
    arrow([(920,389),(920,320)]);arrow([(720,238),(760,238)])
    para('Read, require, route, call and execute are different edges. A delivery record shows which text was supplied; it cannot prove model comprehension.',45,113,1030,13,19)
    finish()

    base('ENGINEERING / CROSS-REPO CHANGE','One task. An exact vector of repository revisions.',
         'Illustrative task: cursor pagination across an API, a Rust SDK and a CLI while preserving existing consumers.')
    box(40,408,320,159,'API service','Producer behavior and schema. Old consumer / new producer is an explicit compatibility case.',True)
    box(400,408,320,159,'Rust SDK','Nominal cursor representation and decoder behavior. Changed bytes stale dependent receipts.')
    box(760,408,320,159,'CLI','Passes opaque cursor values through. Dependency and release ordering stay explicit.')
    arrow([(360,488),(400,488)]);arrow([(720,488),(760,488)])
    rows([
      ('Research and challenge','Resolve assumptions and local conventions. Review alternatives and safe intermediate combinations.'),
      ('Author and verify','Bind tests, proofs, review and documentation to actual targets, tools and fixtures.'),
      ('Land and observe','Recheck upstream and merge candidates. Partial merges follow an admitted continuation or forward fix.')
    ],y=366,h=69,left=290)
    para('A green test for an older vector stays historical. Three green PRs are not an atomic multi-repo release. Uncontrolled external changes remain explicit model inputs.',45,116,1030,14,21)
    finish()

    base('ENGINEERING / EFFECT RECOVERY','A lost response is an unknown outcome.',
         'Persist a unique dispatch claim before external IO. Only its live owner may issue that claim once.')
    box(40,402,300,164,'Prepared','Stable EffectId and operation input; retained evidence and required resource set.')
    box(410,402,300,164,'Dispatch claimed','Durable claim and current authorization stamp. The physical call may or may not have started.',True)
    box(780,402,300,164,'Observed result','Confirm the operation from qualified evidence. A confirmed test may still report TestResult.Fail.')
    arrow([(340,484),(410,484)]);arrow([(710,484),(780,484)])
    box(410,166,300,164,'Unknown','Crash or response loss cannot prove failure. Retain reservations until qualified settlement.',True)
    box(780,166,300,164,'Reconcile','Observe the exact remote identity. Retry only with qualified idempotency or proven no effect.')
    arrow([(560,402),(560,330)]);arrow([(710,248),(780,248)])
    arrow([(930,330),(930,402)])
    para('Even a crash after the claim but before the call recovers as Unknown until reconciled. There is no general exactly-once guarantee for external effects.',45,116,1030,14,21)
    finish()

    base('ENGINEERING / LEARNING','A useful lesson earns a new revision.',
         'Capture observations throughout the run. Repair the current task while evaluating wider applicability separately.')
    box(40,392,320,177,'Observed event','Failure, PR feedback, rollout incident or Tarek\'s correction. Keep source, scope and contrary evidence.')
    box(400,392,320,177,'Candidate change','Knowledge correction, guidance, procedure, tool or scoped preference. Exact proposed bytes.',True)
    box(760,392,320,177,'Frozen evaluation','Baseline, rubric, held-out/adverse cases and cost limits fixed before results.')
    arrow([(360,480),(400,480)]);arrow([(720,480),(760,480)])
    box(40,155,320,165,'Deferred or rejected','Insufficient or contrary evidence remains visible. A hypothesis need not become a global rule.')
    box(400,155,320,165,'Review and admission','Current authority, existing claim rules and preserved policy gate floor. Retain the evidence closure.')
    box(760,155,320,165,'Published revision','Existing Git publication CAS selects it. Future runs adopt; active runs migrate explicitly.',True)
    arrow([(920,392),(920,355),(560,355),(560,320)])
    arrow([(400,238),(360,238)]);arrow([(720,238),(760,238)])
    para('Learning cannot modify its own governing evaluator, remove required gates or grant itself permission. Empirical usefulness and formal admission safety have separate evidence.',45,113,1030,13,19)
    finish()

    base('ENGINEERING / PROPOSED CALCULUS','Three judgments, one explicit proof boundary.',
         'Conservative extension of semantic 2.0.0. Existing T01-T22 remain unchanged; E01-E12 are new, unproved targets.')
    box(40,406,320,161,'Usable knowledge','Which claim is supported and applicable in this exact named context?',True)
    box(400,406,320,161,'Admissible action','Is this step enabled for the actual subject, current grant and owned resources?')
    box(760,406,320,161,'Admissible improvement','Can this evaluated delta enter a new release without weakening the governing rules?')
    rows([
      ('Lean E01-E12','Preservation, conservative extension, binding, capability/gate-floor constraints and conditional compatibility.'),
      ('Verus V16-V21','Actual shipping decisions and safe caller conversions; no unverified easier twin.'),
      ('TLA+ / TLC / TLAPS','M12-M16 plus bridges to existing models. Include time, revocation, upstream change and delayed external completion.')
    ],y=367,h=70,left=250)
    para('WF31 aggregates completed proofs and refinement at P11. Earlier stages contribute scoped receipts. No claim of infallible agents, universally useful lessons or verified arbitrary scripts.',45,113,1030,13,19)
    finish()

    base('ENGINEERING / HUMAN DOSSIER','A readable answer to: what is happening, and why?',
         'Synthetic Obsidian design preview. The prepared document carries both corpus and activity read pins.')
    c.setFillColor(HexColor('#FFFFFF'));c.roundRect(40,105,1040,464,12,fill=1,stroke=0)
    c.setFillColor(color('navy'));c.roundRect(40,105,225,464,12,fill=1,stroke=0)
    para('WRITFOLD',61,540,183,13,bold=True,fill='mint')
    for y,t in [(493,'Change overview'),(451,'Assumptions'),(409,'Repositories'),(367,'Checks and reviews'),(325,'Effect recovery'),(283,'Learning proposals'),(218,'Evidence'),(176,'Human notes')]:
        para(t,61,y,183,12,18,fill='pale')
    para('RUN DEMO-17 / CORPUS R42 / ACTIVITY THROUGH EVENT 38',294,538,748,10,15,fill='muted')
    para('Cursor pagination',294,505,740,27,34,bold=True)
    para('Status: blocked on compatibility evidence',294,457,740,16,23,bold=True,fill='amber')
    para('The old SDK rejects the proposed response field. The API change is being revised; the earlier green CLI check does not cover the new repository vector.',294,416,740,14,21)
    c.setFillColor(color('amberbg'));c.roundRect(294,278,744,65,8,fill=1,stroke=0)
    para('Assumption refuted',310,328,710,13,bold=True,fill='amber')
    para('Old consumers tolerate added fields. Reproduction and decoder source retained.',310,303,710,11.5,17)
    para('Next permitted work',294,253,730,17,bold=True)
    para('Revise compatibility strategy; rerun affected checks; update the linked design and PRs.',294,219,730,13,20)
    para('Learning proposal: test supported old consumers for response-schema extensions.',294,172,730,12,18,fill='teal')
    para('Projection lag and missing evidence remain visible. Editing this note does not admit knowledge or authorize an effect.',45,82,1030,12,18,fill='muted')
    finish()
