---------------------- MODULE M06MigrationSelection ----------------------
EXTENDS Naturals, Sequences, FiniteSets, TLAPS

(***************************************************************************
Runtime map: inventory/dry-run -> Inventory/DryRun; isolated migration/restore
filesystem -> Stage/VerifyStage; durable exact intent -> RetainIntent; conditional
completed ref -> Apply; interrupted operation -> Recover/Replay. HistoricalRead
is a read, never selection. Forward selection and restoration publish fresh v3
snapshots; restoration copies accounted original bytes into a separate owned
staging destination, never overwrites the current KB with an old database.

Trust boundary: canonical identities denote exact immutable bytes; authenticated
completed schema-2 history keeps its original domain. Crypto, applicable policy,
ancestry/prefix validation, inventory enumeration (including unknown sections),
independent restore readback and atomic conditional publication are validator /
effect contracts, not caller-supplied booleans. Checks below are observations of
those validators, not approvals. Imported legacy material has NO knowledge
admission authority: selected claims originate only in authenticated history;
prose, logs and accepted flags remain inventoried material. A real adapter must
validate signatures and byte-chain prefixes, not accept integer ordering alone.

No fairness or eventual IO success is assumed. Explicit stutter admits terminal
states. Source proof obligations below have no qualification receipt attached.
***************************************************************************)
CONSTANTS WriterCount, InputCount, ContextCount
Bounds == /\ WriterCount \in Nat /\ WriterCount >= 2
          /\ InputCount \in Nat /\ InputCount >= 8
          /\ ContextCount \in Nat /\ ContextCount >= 2
ASSUME ParameterBounds == Bounds
USE ParameterBounds DEF Bounds
Writers == 1..WriterCount
Inputs == 1..InputCount
Releases == 0..InputCount
Ops == 1..(InputCount-1)
Contexts == 1..ContextCount
OpOf(i) == IF i <= 2 THEN 1 ELSE i-1
BaseOf(i) == IF i <= 2 \/ i = 4 THEN 0 ELSE i-2
SchemaOf(i) == IF i = 0 \/ i = InputCount THEN 2 ELSE 3
Kind(i) == IF i \in {1,2,4} THEN "migration"
           ELSE IF i = 5 THEN "selection"
           ELSE IF i = 7 THEN "restoration" ELSE "correction"
Chain(i) == IF i = 0 THEN <<0>>
            ELSE IF i = 2 THEN <<0,2>>
            ELSE IF i % 2 = 1
                 THEN [n \in 1..((i+3) \div 2) |-> IF n=1 THEN 0 ELSE 2*n-3]
                 ELSE [n \in 1..(i \div 2) |-> IF n=1 THEN 0 ELSE 2*n]
Members(s) == {s[n] : n \in 1..Len(s)}
Prefix(a,b) == Len(a) <= Len(b) /\ \A n \in 1..Len(a) : a[n] = b[n]
Corrections(i) == Members(Chain(i)) \cap {3,6}
Claims == {"old-supported", "independent"}
Selected(i) == IF i \in {3,6} THEN {"independent"} ELSE Claims
Family(i) == [c \in Contexts |-> [id |-> c, snapshot |-> i, time |-> i,
  policy |-> IF i=0 THEN "legacy-policy" ELSE "current-policy",
  source |-> IF i=0 THEN "legacy-source" ELSE "current-source", use |-> c]]
Record(i) == [snapshot |-> i, schema |-> SchemaOf(i), history |-> Chain(i),
  corrections |-> Corrections(i), selected |-> Selected(i), contexts |-> Family(i)]
Files == {"prose", "unknown-section", "captured-log", "accepted-flag"}
Inventory(v) == [f \in Files |-> <<f,v>>]
Disposition(f) == CASE f="prose" -> "imported"
  [] f="unknown-section" -> "preserved" [] f="captured-log" -> "quarantined"
  [] OTHER -> "rejected"
Reason(f) == CASE f="prose" -> "unapproved-source-only"
  [] f="unknown-section" -> "opaque-byte-preservation"
  [] f="captured-log" -> "untrusted-log-not-evidence"
  [] OTHER -> "legacy-accepted-is-not-approval"
InventoryOf(i) == IF i % 2 = 1 THEN 0 ELSE 1
Accounting(v) == [f \in Files |-> [bytes |-> Inventory(v)[f],
  disposition |-> Disposition(f), reason |-> Reason(f)]]
Input(i) == [canonicalBytes |-> i, operation |-> OpOf(i), predecessor |-> BaseOf(i),
  schema |-> SchemaOf(i), domain |-> IF SchemaOf(i)=3 THEN "kctx-operation-v3"
             ELSE "kctx-operation-v2", inventory |-> Inventory(InventoryOf(i)),
  target |-> 0, kind |-> Kind(i), snapshot |-> i, contexts |-> Family(i),
  approvalBytes |-> <<"migration-selection-approval",i>>]
Checks == {"signature","policy","ancestry","inventory","readback"}
Phases == {"idle","planned","intent","staged","verified","applied","done","failed","down"}
Results == {"none","pending","complete","historical","conflict","refused","unknown"}
LegacyInput == [canonicalBytes |-> <<"legacy-operation",0>>, schema |-> 2,
  domain |-> "kctx-operation-v2", approvalBytes |-> <<"legacy-approval",0>>]
LegacyInputs == {LegacyInput, [LegacyInput EXCEPT !.approvalBytes = <<"tampered",0>>]}
VARIABLE m
vars == <<m>>
Durable == <<m.head,m.retained,m.records,m.original,m.byteRoots,m.intent,
             m.staged,m.verified,m.accounts,m.applied,m.receipts,m.restored>>
Visible == <<m.head,m.retained,m.records>>
StateShape == m = [head |-> m.head, retained |-> m.retained, records |-> m.records,
  inventory |-> m.inventory, original |-> m.original, byteRoots |-> m.byteRoots,
  intent |-> m.intent, staged |-> m.staged, verified |-> m.verified,
  applied |-> m.applied, receipts |-> m.receipts, accounts |-> m.accounts,
  restored |-> m.restored, observed |-> m.observed, request |-> m.request,
  pc |-> m.pc, result |-> m.result, read |-> m.read]
HistoryTyped(head,retained,records) ==
  /\ head \in Releases /\ retained \subseteq Releases
  /\ records \in [Releases -> [snapshot : Releases, schema : {2,3},
       history : Seq(Releases), corrections : SUBSET {3,6}, selected : SUBSET Claims,
       contexts : [Contexts -> [id : Contexts, snapshot : Releases, time : Releases,
         policy : {"legacy-policy","current-policy"},
         source : {"legacy-source","current-source"}, use : Contexts]]]]
InventoryTyped(inventory,original,byteRoots,accounts) ==
  /\ inventory \in {0,1} /\ original=Inventory(0) /\ byteRoots \subseteq {0,1}
  /\ DOMAIN accounts=Inputs
  /\ accounts=[i \in Inputs |-> accounts[i]]
  /\ (\A i \in Inputs :
       /\ DOMAIN accounts[i] \subseteq Files
       /\ accounts[i]=[f \in DOMAIN accounts[i] |-> accounts[i][f]]
       /\ (\A f \in DOMAIN accounts[i] : accounts[i][f] \in
            [bytes : Files \X {0,1},
             disposition : {"imported","preserved","quarantined","rejected"},
             reason : {"unapproved-source-only","opaque-byte-preservation",
                       "untrusted-log-not-evidence","legacy-accepted-is-not-approval"}]))
StagingTyped(intent,staged,verified,applied,receipts,restored) ==
  /\ intent \in [Ops -> Releases] /\ staged \subseteq Inputs
  /\ verified \subseteq Inputs /\ applied \subseteq Inputs
  /\ receipts \subseteq Inputs /\ restored \subseteq Inputs
ControlTyped(observed,request,pc,result,reads) ==
  /\ observed \in [Inputs -> SUBSET Checks] /\ request \in [Writers -> Releases]
  /\ pc \in [Writers -> Phases] /\ result \in [Writers -> Results]
  /\ reads \in [Writers -> Releases]
HistoryTypes == HistoryTyped(m.head,m.retained,m.records)
InventoryTypes == InventoryTyped(m.inventory,m.original,m.byteRoots,m.accounts)
StagingTypes == StagingTyped(m.intent,m.staged,m.verified,m.applied,m.receipts,m.restored)
ControlTypes == ControlTyped(m.observed,m.request,m.pc,m.result,m.read)
TypeOK == StateShape /\ HistoryTypes /\ InventoryTypes /\ StagingTypes /\ ControlTypes

(* Validator evidence binds the complete exact input and the retained byte chain.
   The ancestry check is both a successful external authentication observation
   and an internal exact prefix/retained-record relation. No arbitrary bool is
   an action argument granting permission to write. *)
Authority(i) == /\ SchemaOf(i)=3 /\ {"signature","policy"} \subseteq m.observed[i]
  /\ Input(i).approvalBytes = <<"migration-selection-approval",i>>
AuthenticatedPrefix(i) == /\ "ancestry" \in m.observed[i]
  /\ Input(i).target \in m.retained /\ BaseOf(i) \in m.retained
  /\ m.records[0] = Record(0) /\ m.records[BaseOf(i)] = Record(BaseOf(i))
  /\ Prefix(m.records[0].history,m.records[BaseOf(i)].history)
ExactInventory(i) == /\ "inventory" \in m.observed[i]
  /\ Input(i).inventory = Inventory(m.inventory)
Valid(i) == Authority(i) /\ AuthenticatedPrefix(i) /\ ExactInventory(i)
BoundIntent(i) == m.intent[OpOf(i)] = i
CompleteAccounting(i) == m.accounts[i] = Accounting(InventoryOf(i))
StageReady(i) == /\ i \in m.staged /\ BoundIntent(i) /\ CompleteAccounting(i)
  /\ InventoryOf(i) \in m.byteRoots
HistoryValid(head,retained,records,applied) ==
  /\ 0 \in retained /\ head \in retained /\ applied=retained \ {0}
  /\ (\A i \in retained : records[i]=Record(i))
  /\ retained=Members(Chain(head))
  /\ (\A i \in applied : SchemaOf(i)=3 /\ BaseOf(i) \in retained)
StageValid(byteRoots,intent,staged,verified,applied,receipts,restored,accounts) ==
  /\ 0 \in byteRoots
  /\ (\A o \in Ops : intent[o] # 0 => OpOf(intent[o])=o)
  /\ (\A i \in staged :
       /\ intent[OpOf(i)]=i /\ accounts[i]=Accounting(InventoryOf(i))
       /\ InventoryOf(i) \in byteRoots /\ SchemaOf(i)=3)
  /\ verified \subseteq staged /\ applied \subseteq verified
  /\ receipts \subseteq applied /\ restored \subseteq staged
  /\ (\A i \in restored : Kind(i)="restoration")
Completed(request,result,receipts,records) == \A w \in Writers :
  /\ (result[w]="complete" => request[w] \in receipts)
  /\ (result[w]="historical" => request[w]=0 /\ records[0]=Record(0))
HistorySound == HistoryValid(m.head,m.retained,m.records,m.applied)
StageSound == StageValid(m.byteRoots,m.intent,m.staged,m.verified,m.applied,
                        m.receipts,m.restored,m.accounts)
CompletionSound == Completed(m.request,m.result,m.receipts,m.records)
(* Query projection keeps a selected-but-retracted claim from resurrecting its
   invalidated support. Historical-as-of reads use that historical record's
   corrections; current selection always uses the current record's corrections. *)
Supported(i) == m.records[i].selected \
  (IF m.records[i].corrections # {} THEN {"old-supported"} ELSE {})
DryRunReport == [base |-> m.head, inventory |-> Inventory(m.inventory),
  accounting |-> Accounting(m.inventory), approvedKnowledge |-> {}]
(* Container versions are independent of new operation/approval schema 3. *)
ContainerSchemas == [migration |-> 2, backup |-> 2, policy |-> 2]
Inv == TypeOK /\ HistorySound /\ StageSound /\ CompletionSound
Safety ==
  /\ (\A i \in m.applied :
       /\ CompleteAccounting(i)
       /\ BoundIntent(i)
       /\ Input(i).schema=3
       /\ m.records[i].history=Chain(i)
       /\ m.records[i].corrections=Corrections(i)
       /\ m.records[i].contexts=Family(i)
       /\ m.records[i].selected=Selected(i))
  /\ m.original=Inventory(0)
  /\ 0 \in m.byteRoots
  /\ m.records[0]=Record(0)
  /\ m.receipts \subseteq m.applied
  /\ CompletionSound
  /\ (\A i \in m.applied : m.records[i].corrections # {} =>
       "old-supported" \notin Supported(i))

Init == m = [head |-> 0, retained |-> {0}, records |-> [i \in Releases |-> Record(0)],
  inventory |-> 0, original |-> Inventory(0), byteRoots |-> {0},
  intent |-> [o \in Ops |-> 0], staged |-> {}, verified |-> {}, applied |-> {},
  receipts |-> {}, accounts |-> [i \in Inputs |-> Accounting(InventoryOf(i))],
  restored |-> {}, observed |-> [i \in Inputs |-> Checks],
  request |-> [w \in Writers |-> 0], pc |-> [w \in Writers |-> "idle"],
  result |-> [w \in Writers |-> "none"], read |-> [w \in Writers |-> 0]]

DryRun == UNCHANGED m
HistoricalRead(w,i) == /\ i \in m.retained /\ m'=[m EXCEPT !.read[w]=i]
(* Exact completed v2 journal replay is a read. It cannot resume a v2 write or
   mint a new v3 receipt. LegacyInput denotes retained authenticated old bytes. *)
LegacyReplay(w,b) == /\ b=LegacyInput /\ 0 \in m.retained
  /\ m.records[0]=Record(0) /\ m.pc[w] \in {"idle","done","failed"}
  /\ m'=[m EXCEPT !.request[w]=0, !.pc[w]="done", !.result[w]="historical"]
InventoryChange(v) == /\ m'=[m EXCEPT !.inventory=v, !.byteRoots=@ \cup {v}]
Observe(i,a) == /\ a \subseteq Checks /\ m'=[m EXCEPT !.observed[i]=a]
Start(w,i) == /\ m.pc[w] \in {"idle","done","failed"}
  /\ m'=[m EXCEPT !.request[w]=i, !.pc[w]="planned", !.result[w]="pending"]
RetainIntent(w) == LET i==m.request[w] IN
  /\ i \in Inputs /\ m.pc[w]="planned" /\ Valid(i) /\ m.head=BaseOf(i)
  /\ m.intent[OpOf(i)] \in {0,i}
  /\ m'=[m EXCEPT !.intent[OpOf(i)]=i, !.pc[w]="intent"]
Stage(w) == LET i==m.request[w] IN
  /\ i \in Inputs /\ m.pc[w]="intent" /\ BoundIntent(i) /\ Valid(i)
  /\ m'=[m EXCEPT !.staged=@ \cup {i}, !.accounts[i]=Accounting(InventoryOf(i)),
       !.byteRoots=@ \cup {InventoryOf(i)}, !.pc[w]="staged"]
RestoreOriginal(w) == LET i==m.request[w] IN
  /\ i \in Inputs /\ Kind(i)="restoration" /\ m.pc[w]="staged" /\ StageReady(i)
  /\ m'=[m EXCEPT !.restored=@ \cup {i}]
VerifyStage(w) == LET i==m.request[w] IN
  /\ i \in Inputs /\ m.pc[w]="staged" /\ StageReady(i) /\ Valid(i)
  /\ "readback" \in m.observed[i]
  /\ (Kind(i)="restoration" => i \in m.restored)
  /\ m'=[m EXCEPT !.verified=@ \cup {i}, !.pc[w]="verified"]
Candidate(i) == [snapshot |-> i, schema |-> 3,
  history |-> Append(m.records[m.head].history,i),
  corrections |-> m.records[m.head].corrections \cup (IF Kind(i)="correction" THEN {i} \cap {3,6} ELSE {}),
  selected |-> IF Kind(i) \in {"selection","restoration"} THEN m.records[0].selected
              ELSE Selected(i),
  contexts |-> IF Kind(i) \in {"selection","restoration"}
     THEN [c \in Contexts |-> [m.records[m.head].contexts[c] EXCEPT !.snapshot=i, !.time=i]]
     ELSE Family(i)]
Apply(w) == LET i==m.request[w] IN
  /\ i \in Inputs /\ m.pc[w]="verified" /\ i \in m.verified /\ StageReady(i)
  /\ Valid(i) /\ "readback" \in m.observed[i]
  /\ m.head=BaseOf(i) /\ i \notin m.retained
  /\ m'=[m EXCEPT !.head=i, !.retained=@ \cup {i}, !.applied=@ \cup {i},
       !.records[i]=Candidate(i), !.pc[w]="applied"]
RecordReceipt(w) == LET i==m.request[w] IN
  /\ i \in Inputs /\ m.pc[w]="applied" /\ i \in m.applied /\ BoundIntent(i)
  /\ m'=[m EXCEPT !.receipts=@ \cup {i}, !.pc[w]="done", !.result[w]="complete"]
Replay(w) == LET i==m.request[w] IN
  /\ i \in Inputs /\ m.pc[w]="planned" /\ i \in m.applied /\ BoundIntent(i)
  /\ m.records[i]=Record(i) /\ Input(i).canonicalBytes=i
  /\ m'=[m EXCEPT !.receipts=@ \cup {i}, !.pc[w]="done", !.result[w]="complete"]
Recover(w) == LET i==m.request[w] IN
  /\ i \in Inputs /\ m.pc[w]="planned" /\ i \notin m.applied
  /\ BoundIntent(i) /\ Valid(i) /\ m.head=BaseOf(i)
  /\ m'=[m EXCEPT !.pc[w]=IF i \in m.staged THEN "staged" ELSE "intent"]
Conflict(w) == LET i==m.request[w] IN
  /\ i \in Inputs /\ m.pc[w] \in {"planned","intent","staged","verified"}
  /\ (m.intent[OpOf(i)] \notin {0,i} \/
      (i \notin m.applied /\ (m.head # BaseOf(i) \/ ~ExactInventory(i))))
  /\ m'=[m EXCEPT !.pc[w]="failed", !.result[w]="conflict"]
Refuse(w) == LET i==m.request[w] IN
  /\ i \in Inputs /\ m.pc[w] \in {"planned","intent","staged","verified"}
  /\ (~Authority(i) \/ ~AuthenticatedPrefix(i) \/ "readback" \notin m.observed[i])
  /\ m'=[m EXCEPT !.pc[w]="failed", !.result[w]="refused"]
FailIO(w) == /\ m.pc[w] \in {"planned","intent","staged","verified","applied"}
  /\ m'=[m EXCEPT !.pc[w]="failed", !.result[w]="unknown"]
Crash(w) == /\ m.pc[w] # "down"
  /\ m'=[m EXCEPT !.pc[w]="down", !.result[w]="unknown"]
Restart(w) == /\ m.pc[w]="down"
  /\ m'=[m EXCEPT !.pc[w]="idle", !.request[w]=0, !.result[w]="none"]
Stutter == UNCHANGED m
CoreNext ==
  \/ DryRun
  \/ Stutter
  \/ (\E w \in Writers, i \in Inputs : Start(w,i))
  \/ (\E w \in Writers, i \in Releases : HistoricalRead(w,i))
  \/ (\E w \in Writers, b \in LegacyInputs : LegacyReplay(w,b))
  \/ (\E v \in {0,1} : InventoryChange(v))
  \/ (\E w \in Writers :
       RetainIntent(w) \/ Stage(w) \/ RestoreOriginal(w)
       \/ VerifyStage(w) \/ Apply(w) \/ RecordReceipt(w) \/ Replay(w) \/ Recover(w)
       \/ Conflict(w) \/ Refuse(w) \/ FailIO(w) \/ Crash(w) \/ Restart(w))
Next == CoreNext \/ \E i \in Inputs, a \in SUBSET Checks : Observe(i,a)
Spec == Init /\ [][Next]_vars

(* Finite fixtures replace arbitrary observation combinations with declared
   representative vectors, NOT a global error cap. Independent simultaneous
   signature/ancestry/inventory/readback failures on competing operations remain
   present while another operation is valid. No scheduling/phase/generation bound.
   Inductive Next above retains unrestricted per-input observation changes. *)
Profiles == { {}, {<<1,"signature">>}, {<<1,"signature">>,<<2,"policy">>},
  {<<1,"ancestry">>,<<2,"inventory">>,<<4,"readback">>},
  {<<3,"readback">>,<<5,"policy">>}, {<<5,"ancestry">>,<<7,"readback">>} }
ObserveProfile(f) == m'=[m EXCEPT !.observed=
  [i \in Inputs |-> {c \in Checks : <<i,c>> \notin f}]]
FiniteNext == CoreNext \/ \E f \in Profiles : ObserveProfile(f)
FiniteSpec == Init /\ [][FiniteNext]_vars

(* Negative controls preserve types and otherwise perform the actual operation.
   They target Safety rather than a strengthened-only bookkeeping invariant. *)
MutantDropDisposition(w) == LET i==m.request[w] IN
  /\ i \in Inputs /\ i \in m.applied /\ m.pc[w]="applied"
  /\ m'=[m EXCEPT !.accounts[i]=[f \in Files \ {"accepted-flag"} |-> m.accounts[i][f]]]
MutantRestoreOld(w) == LET i==m.request[w] IN
  /\ i \in Inputs /\ Kind(i) \in {"selection","restoration"}
  /\ i \in m.applied /\ m.pc[w]="applied"
  /\ m'=[m EXCEPT !.records[i].corrections=m.records[0].corrections,
       !.records[i].contexts=m.records[0].contexts]
DropDispositionNext == FiniteNext \/ \E w \in Writers : MutantDropDisposition(w)
RestoreOldNext == FiniteNext \/ \E w \in Writers : MutantRestoreOld(w)
DropDispositionSpec == Init /\ [][DropDispositionNext]_vars
RestoreOldSpec == Init /\ [][RestoreOldNext]_vars

(* Pure field-group predicates make unchanged groups congruence goals; only the
   group actually changed by an action is expanded in its preservation proof. *)
ChainLength(i) == IF i=0 THEN 1 ELSE IF i=2 THEN 2
  ELSE IF i % 2=1 THEN (i+3) \div 2 ELSE i \div 2
THEOREM InputShape == \A i \in Inputs :
  /\ i \in Releases /\ OpOf(i) \in Ops /\ BaseOf(i) \in Releases
  /\ BaseOf(i)<i /\ InventoryOf(i) \in {0,1}
BY SMT DEF Inputs, Releases, Ops, OpOf, BaseOf, InventoryOf
THEOREM ChainShape == \A i \in Releases :
  /\ ChainLength(i) \in Nat /\ ChainLength(i)>0
  /\ Chain(i) \in [1..ChainLength(i) -> Releases]
  /\ Len(Chain(i))=ChainLength(i) /\ Chain(i) \in Seq(Releases)
<1>1. SUFFICES ASSUME NEW i \in Releases
      PROVE /\ ChainLength(i) \in Nat /\ ChainLength(i)>0
            /\ Chain(i) \in [1..ChainLength(i) -> Releases]
            /\ Len(Chain(i))=ChainLength(i) /\ Chain(i) \in Seq(Releases)
  OBVIOUS
<1>2. CASE i=0
  BY SMT, <1>2 DEF Chain, ChainLength, Releases
<1>3. CASE i=2
  BY SMT, <1>3 DEF Chain, ChainLength, Releases
<1>4. CASE i # 0 /\ i # 2 /\ i % 2=1
  BY SMT, <1>4 DEF Chain, ChainLength, Releases
<1>5. CASE i # 0 /\ i # 2 /\ i % 2 # 1
  BY SMT, <1>5 DEF Chain, ChainLength, Releases
<1> QED BY <1>2, <1>3, <1>4, <1>5
THEOREM AppendMembers == \A s \in Seq(Releases), i \in Releases :
  Members(Append(s,i)) = Members(s) \cup {i}
BY SMT DEF Members
THEOREM ChainLengthExtension == \A i \in Inputs :
  ChainLength(i) = ChainLength(BaseOf(i)) + 1
BY SMT DEF Inputs, ChainLength, BaseOf
THEOREM ChainLast == \A i \in Inputs : Chain(i)[ChainLength(i)] = i
BY SMT, ChainShape, InputShape
  DEF ChainShape, InputShape, Chain, ChainLength, Inputs, Releases
THEOREM ChainPrefixIndices == \A i \in Inputs :
  \A n \in 1..ChainLength(BaseOf(i)) : Chain(i)[n] = Chain(BaseOf(i))[n]
<1>1. ASSUME NEW i \in Inputs, NEW n \in 1..ChainLength(BaseOf(i))
      PROVE Chain(i)[n]=Chain(BaseOf(i))[n]
  <2> USE i \in Inputs, n \in 1..ChainLength(BaseOf(i))
  <2>1. CASE i \in {1,2,4}
    BY SMT, <2>1 DEF Chain, ChainLength, BaseOf, Inputs
  <2>2. CASE i \notin {1,2,4} /\ i % 2=1
    <3>1. i >= 3 /\ BaseOf(i)=i-2 /\ (i-2) % 2=1 /\
      n \in 1..((i+1) \div 2) /\ n \in 1..((i+3) \div 2)
      BY SMT, <2>2 DEF Inputs, BaseOf, ChainLength
    <3>2. QED BY SMT, <2>2, <3>1 DEF Chain, Inputs
  <2>3. CASE i \notin {1,2,4} /\ i % 2 # 1
    <3>1. i >= 6 /\ BaseOf(i)=i-2 /\ (i-2) % 2=0 /\
      n \in 1..((i-2) \div 2) /\ n \in 1..(i \div 2)
      BY SMT, <2>3 DEF Inputs, BaseOf, ChainLength
    <3>2. QED BY SMT, <2>3, <3>1 DEF Chain, Inputs
  <2>4. QED BY <2>1, <2>2, <2>3
<1>2. QED BY <1>1
THEOREM ChainMemberBound == \A i \in Releases :
  \A j \in Members(Chain(i)) : j <= i
BY SMT, ChainShape DEF ChainShape, Chain, ChainLength, Members, Releases
THEOREM ChainAppend == \A i \in Inputs : Chain(i)=Append(Chain(BaseOf(i)),i)
<1>1. ASSUME NEW i \in Inputs PROVE Chain(i)=Append(Chain(BaseOf(i)),i)
  <2>1. i \in Releases /\ BaseOf(i) \in Releases
    BY <1>1, InputShape DEF InputShape
  <2>2. Chain(i) \in Seq(Releases) /\ Chain(BaseOf(i)) \in Seq(Releases)
    BY <2>1, ChainShape DEF ChainShape
  <2>3. Len(Chain(i))=Len(Chain(BaseOf(i)))+1
    BY SMT, <1>1, <2>1, ChainShape, ChainLengthExtension
      DEF ChainShape, ChainLengthExtension
  <2>4. \A n \in 1..Len(Chain(BaseOf(i))) :
    Chain(i)[n]=Chain(BaseOf(i))[n]
    BY SMT, <1>1, <2>1, ChainShape, ChainPrefixIndices
      DEF ChainShape, ChainPrefixIndices
  <2>5. Chain(i)[Len(Chain(i))]=i
    BY SMT, <1>1, <2>1, ChainShape, ChainLast DEF ChainShape, ChainLast
  <2>6. QED BY SMT, <2>1, <2>2, <2>3, <2>4, <2>5
<1>2. QED BY <1>1
THEOREM ChainExtension == \A i \in Inputs :
  /\ Chain(i)=Append(Chain(BaseOf(i)),i)
  /\ Members(Chain(i))=Members(Chain(BaseOf(i))) \cup {i}
  /\ BaseOf(i)<i /\ i \notin Members(Chain(BaseOf(i)))
<1>1. \A i \in Inputs :
  Chain(i)=Append(Chain(BaseOf(i)),i)
  BY ChainAppend DEF ChainAppend
<1>2. \A i \in Inputs :
  Members(Chain(i))=Members(Chain(BaseOf(i))) \cup {i}
  BY SMT, <1>1, AppendMembers, ChainShape, InputShape
    DEF AppendMembers, ChainShape, InputShape
<1>3. \A i \in Inputs : BaseOf(i)<i /\ i \notin Members(Chain(BaseOf(i)))
  BY SMT, InputShape, ChainMemberBound
    DEF InputShape, ChainMemberBound, Inputs, Releases
<1>4. QED BY <1>1, <1>2, <1>3
THEOREM CorrectionExtension == \A i \in Inputs : Corrections(i)=
  Corrections(BaseOf(i)) \cup (IF Kind(i)="correction" THEN {i} \cap {3,6} ELSE {})
BY SMT, ChainExtension DEF ChainExtension, Corrections, Kind
THEOREM RecordTyped == \A i \in Releases : Record(i) \in
  [snapshot : Releases, schema : {2,3}, history : Seq(Releases),
   corrections : SUBSET {3,6}, selected : SUBSET Claims,
   contexts : [Contexts -> [id : Contexts, snapshot : Releases, time : Releases,
    policy : {"legacy-policy","current-policy"},
    source : {"legacy-source","current-source"}, use : Contexts]]]
BY SMT, ChainShape DEF ChainShape, Record, SchemaOf, Corrections, Selected, Family, Claims
THEOREM HistoryRecordReplacementTyped == \A h,i \in Releases : \A retained,records :
  HistoryTyped(h,retained,records) =>
    HistoryTyped(i,retained \cup {i},[records EXCEPT ![i]=Record(i)])
<1>1. ASSUME NEW h \in Releases, NEW i \in Releases, NEW retained, NEW records,
        HistoryTyped(h,retained,records)
      PROVE HistoryTyped(i,retained \cup {i},[records EXCEPT ![i]=Record(i)])
  <2> USE h \in Releases, i \in Releases, HistoryTyped(h,retained,records)
  <2> DEFINE R == [snapshot : Releases, schema : {2,3}, history : Seq(Releases),
    corrections : SUBSET {3,6}, selected : SUBSET Claims,
    contexts : [Contexts -> [id : Contexts, snapshot : Releases, time : Releases,
      policy : {"legacy-policy","current-policy"},
      source : {"legacy-source","current-source"}, use : Contexts]]]
  <2>1. records \in [Releases -> R] /\ Record(i) \in R
    BY RecordTyped DEF RecordTyped, HistoryTyped, R
  <2>2. [records EXCEPT ![i]=Record(i)] =
    [j \in Releases |-> IF j=i THEN Record(i) ELSE records[j]]
    BY SMT, <2>1
  <2>3. \A j \in Releases : (IF j=i THEN Record(i) ELSE records[j]) \in R
    BY SMT, <2>1
  <2>4. [records EXCEPT ![i]=Record(i)] \in [Releases -> R]
    BY SMT, <2>2, <2>3
  <2>5. QED BY SMT, <2>4 DEF HistoryTyped, R
<1>2. QED BY <1>1
THEOREM AccountingTyped == \A v \in {0,1} :
  /\ DOMAIN Accounting(v)=Files
  /\ (\A f \in Files : Accounting(v)[f] \in
       [bytes : Files \X {0,1},
        disposition : {"imported","preserved","quarantined","rejected"},
        reason : {"unapproved-source-only","opaque-byte-preservation",
                  "untrusted-log-not-evidence","legacy-accepted-is-not-approval"}])
BY SMT DEF Accounting, Inventory, Disposition, Reason, Files
THEOREM InitPreserves == Init => Inv
<1>1. Init => StateShape /\ HistoryTypes
  BY SMT, RecordTyped DEF RecordTyped, Init, StateShape, HistoryTypes,
    HistoryTyped, Releases
<1>2. Init => InventoryTypes
  BY SMT, AccountingTyped, InputShape DEF AccountingTyped, InputShape, Init,
    InventoryTypes, InventoryTyped, Accounting, InventoryOf, Inventory
<1>3. Init => StagingTypes /\ ControlTypes
  BY SMT DEF Init, StagingTypes, StagingTyped, ControlTypes, ControlTyped,
    Releases, Phases, Results
<1>4. Init => HistorySound
  BY SMT DEF Init, HistorySound, HistoryValid, Chain, Members, Releases
<1>5. Init => StageSound /\ CompletionSound
  BY SMT DEF Init, StageSound, StageValid, CompletionSound, Completed
<1> QED BY <1>1, <1>2, <1>3, <1>4, <1>5 DEF Inv, TypeOK
THEOREM CandidatePreservesHistory == \A i \in Inputs :
  Inv /\ Valid(i) /\ m.head=BaseOf(i) => Candidate(i)=Record(i)
<1>1. \A i \in Inputs : Inv /\ Valid(i) /\ m.head=BaseOf(i) =>
       Candidate(i).history=Record(i).history
  BY SMT, ChainExtension DEF ChainExtension, Inv, HistorySound, HistoryValid,
    Candidate, Record
<1>2. \A i \in Inputs : Inv /\ Valid(i) /\ m.head=BaseOf(i) =>
       Candidate(i).corrections=Record(i).corrections
  BY SMT, CorrectionExtension DEF CorrectionExtension, Inv, HistorySound,
    HistoryValid, Candidate, Record
<1>3. \A i \in Inputs : Inv /\ Valid(i) /\ m.head=BaseOf(i) =>
       Candidate(i).contexts=Record(i).contexts
  BY SMT DEF Inv, HistorySound, HistoryValid, Candidate, Record,
    Family, Kind, BaseOf
<1>4. \A i \in Inputs : Inv /\ Valid(i) /\ m.head=BaseOf(i) =>
       /\ Candidate(i).snapshot=Record(i).snapshot
       /\ Candidate(i).schema=Record(i).schema
       /\ Candidate(i).selected=Record(i).selected
  BY SMT DEF Inv, HistorySound, HistoryValid, Candidate, Record, Selected,
    Kind, Valid, Authority
<1> QED BY SMT, <1>1, <1>2, <1>3, <1>4 DEF Candidate, Record
THEOREM StutterPreserves == Inv /\ Stutter => Inv'
BY SMT DEF Stutter, Inv, TypeOK, StateShape, HistoryTypes, InventoryTypes,
  StagingTypes, ControlTypes, HistorySound, StageSound, CompletionSound
THEOREM DryRunPreserves == Inv /\ DryRun => Inv'
BY StutterPreserves DEF StutterPreserves, DryRun, Stutter
THEOREM DryRunReadOnly == DryRun => UNCHANGED m
BY DEF DryRun
THEOREM HistoricalReadFrame == \A w \in Writers, i \in Releases :
  TypeOK /\ HistoricalRead(w,i) => UNCHANGED Durable
BY SMT DEF TypeOK, StateShape, HistoricalRead, Durable
THEOREM RefusalFrame == \A w \in Writers : TypeOK /\
  (Conflict(w) \/ Refuse(w) \/ FailIO(w) \/ Crash(w) \/ Restart(w)) => UNCHANGED Durable
BY SMT DEF TypeOK, StateShape, Conflict, Refuse, FailIO, Crash, Restart, Durable
THEOREM ReplayFrame == \A w \in Writers : TypeOK /\ Replay(w) => UNCHANGED
  <<m.head,m.retained,m.records,m.original,m.byteRoots,m.intent,m.staged,m.verified,m.accounts,m.applied,m.restored>>
BY SMT DEF TypeOK, StateShape, Replay
THEOREM LegacyPendingRefused == \A w \in Writers :
  Inv /\ m.request[w] \in Inputs /\ SchemaOf(m.request[w])=2 =>
  /\ ~RetainIntent(w) /\ ~Recover(w) /\ ~Apply(w)
BY SMT DEF RetainIntent, Recover, Apply, Valid, Authority
THEOREM ForwardSelectionFrame == \A w \in Writers : LET i==m.request[w] IN
  Inv /\ Apply(w) /\ Kind(i) \in {"selection","restoration"} =>
  /\ m'.records[i].history=Append(m.records[m.head].history,i)
  /\ m'.records[i].corrections=m.records[m.head].corrections
  /\ m'.records[i].selected=m.records[0].selected
  /\ (\A c \in Contexts : m'.records[i].contexts[c]=
       [m.records[m.head].contexts[c] EXCEPT !.snapshot=i, !.time=i])
  /\ (\A c \in Contexts : m'.records[i].contexts[c].time >= m.records[m.head].contexts[c].time)
<1>1. ASSUME NEW w \in Writers, Inv, Apply(w),
        Kind(m.request[w]) \in {"selection","restoration"}
      PROVE /\ m'.records[m.request[w]].history=Append(m.records[m.head].history,m.request[w])
            /\ m'.records[m.request[w]].corrections=m.records[m.head].corrections
            /\ m'.records[m.request[w]].selected=m.records[0].selected
            /\ (\A c \in Contexts : m'.records[m.request[w]].contexts[c]=
                 [m.records[m.head].contexts[c] EXCEPT !.snapshot=m.request[w], !.time=m.request[w]])
            /\ (\A c \in Contexts : m'.records[m.request[w]].contexts[c].time >=
                 m.records[m.head].contexts[c].time)
  <2> USE Inv, Apply(w), Kind(m.request[w]) \in {"selection","restoration"}
  <2>1. m.request[w] \in Inputs /\ m.head=BaseOf(m.request[w]) /\
    m'.records[m.request[w]]=Candidate(m.request[w])
    BY SMT, <1>1, InputShape DEF InputShape, Inv, TypeOK, StateShape,
      HistoryTypes, HistoryTyped, Apply
  <2>2. m.records[m.head]=Record(m.head) /\ m.head<m.request[w]
    BY SMT, <1>1, <2>1, InputShape
      DEF InputShape, Inv, HistorySound, HistoryValid
  <2>3. \A c \in Contexts : m.records[m.head].contexts[c].time=m.head
    BY SMT, <2>2 DEF Record, Family
  <2>4. \A c \in Contexts :
    Candidate(m.request[w]).contexts[c].time=m.request[w]
    BY SMT, <1>1, <2>2 DEF Candidate, Record, Family
  <2>5. m.head \in Nat /\ m.request[w] \in Nat
    BY SMT, <2>1, InputShape DEF InputShape, Inputs, Releases
  <2>6. m.records[m.head].corrections \subseteq {3,6}
    BY SMT DEF Inv, TypeOK, HistoryTypes, HistoryTyped
  <2>7. m'.records[m.request[w]].history=Append(m.records[m.head].history,m.request[w])
    BY SMT, <2>1 DEF Candidate
  <2>8. m'.records[m.request[w]].corrections=m.records[m.head].corrections
    <3>1. Kind(m.request[w]) # "correction"
      BY SMT
    <3>2. m.records[m.head].corrections=Corrections(m.head)
      BY SMT, <2>2 DEF Record
    <3>3. Corrections(m.head) \cup {} = Corrections(m.head)
      BY SMT DEF Corrections
    <3>4. QED BY SMT, <2>1, <3>1, <3>2, <3>3 DEF Candidate
  <2>9. m'.records[m.request[w]].selected=m.records[0].selected
    BY SMT, <2>1 DEF Candidate
  <2>10. \A c \in Contexts : m'.records[m.request[w]].contexts[c]=
    [m.records[m.head].contexts[c] EXCEPT !.snapshot=m.request[w], !.time=m.request[w]]
    BY SMT, <2>1 DEF Candidate
  <2>11. \A c \in Contexts : m'.records[m.request[w]].contexts[c].time >=
    m.records[m.head].contexts[c].time
    BY SMT, <2>1, <2>2, <2>3, <2>4, <2>5
  <2>12. QED BY <2>7, <2>8, <2>9, <2>10, <2>11
<1>2. QED BY <1>1
THEOREM HistoricalReadPreserves == \A w \in Writers, i \in Releases : Inv /\ HistoricalRead(w,i) => Inv'
<1>1. \A w \in Writers, i \in Releases : Inv /\ HistoricalRead(w,i) => TypeOK'
  BY SMT DEF Inv, TypeOK, StateShape, HistoryTypes, InventoryTypes, StagingTypes, ControlTypes, HistoricalRead, ControlTyped, Phases, Results
<1>2. \A w \in Writers, i \in Releases : Inv /\ HistoricalRead(w,i) => HistorySound'
  BY SMT DEF Inv, TypeOK, StateShape, HistorySound, HistoricalRead
<1>3. \A w \in Writers, i \in Releases : Inv /\ HistoricalRead(w,i) => StageSound'
  BY SMT DEF Inv, TypeOK, StateShape, StageSound, HistoricalRead
<1>4. \A w \in Writers, i \in Releases : Inv /\ HistoricalRead(w,i) => CompletionSound'
  BY SMT DEF Inv, TypeOK, StateShape, CompletionSound, HistoricalRead
<1> QED BY <1>1, <1>2, <1>3, <1>4 DEF Inv
THEOREM InventoryChangePreserves == \A v \in {0,1} : Inv /\ InventoryChange(v) => Inv'
<1>1. \A v \in {0,1} : Inv /\ InventoryChange(v) => TypeOK'
  BY SMT DEF Inv, TypeOK, StateShape, HistoryTypes, InventoryTypes, StagingTypes, ControlTypes, InventoryChange, InventoryTyped
<1>2. \A v \in {0,1} : Inv /\ InventoryChange(v) => HistorySound'
  BY SMT DEF Inv, TypeOK, StateShape, HistorySound, InventoryChange
<1>3. \A v \in {0,1} : Inv /\ InventoryChange(v) => StageSound'
  BY SMT DEF Inv, TypeOK, StateShape, StageSound, InventoryChange, StageValid, StageReady, CompleteAccounting, BoundIntent, StagingTypes, StagingTyped
<1>4. \A v \in {0,1} : Inv /\ InventoryChange(v) => CompletionSound'
  BY SMT DEF Inv, TypeOK, StateShape, CompletionSound, InventoryChange
<1> QED BY <1>1, <1>2, <1>3, <1>4 DEF Inv
THEOREM ObservePreserves == \A i \in Inputs, a \in SUBSET Checks : Inv /\ Observe(i,a) => Inv'
<1>1. \A i \in Inputs, a \in SUBSET Checks : Inv /\ Observe(i,a) => TypeOK'
  BY SMT DEF Inv, TypeOK, StateShape, HistoryTypes, InventoryTypes, StagingTypes, ControlTypes, Observe, ControlTyped, Phases, Results
<1>2. \A i \in Inputs, a \in SUBSET Checks : Inv /\ Observe(i,a) => HistorySound'
  BY SMT DEF Inv, TypeOK, StateShape, HistorySound, Observe
<1>3. \A i \in Inputs, a \in SUBSET Checks : Inv /\ Observe(i,a) => StageSound'
  BY SMT DEF Inv, TypeOK, StateShape, StageSound, Observe
<1>4. \A i \in Inputs, a \in SUBSET Checks : Inv /\ Observe(i,a) => CompletionSound'
  BY SMT DEF Inv, TypeOK, StateShape, CompletionSound, Observe
<1> QED BY <1>1, <1>2, <1>3, <1>4 DEF Inv
THEOREM StartPreserves == \A w \in Writers, i \in Inputs : Inv /\ Start(w,i) => Inv'
<1>1. \A w \in Writers, i \in Inputs : Inv /\ Start(w,i) => TypeOK'
  BY SMT DEF Inv, TypeOK, StateShape, HistoryTypes, InventoryTypes, StagingTypes, ControlTypes, Start, ControlTyped, Phases, Results, Inputs, Releases, OpOf, Ops
<1>2. \A w \in Writers, i \in Inputs : Inv /\ Start(w,i) => HistorySound'
  BY SMT DEF Inv, TypeOK, StateShape, HistorySound, Start
<1>3. \A w \in Writers, i \in Inputs : Inv /\ Start(w,i) => StageSound'
  BY SMT DEF Inv, TypeOK, StateShape, StageSound, Start
<1>4. \A w \in Writers, i \in Inputs : Inv /\ Start(w,i) => CompletionSound'
  BY SMT DEF Inv, TypeOK, StateShape, CompletionSound, Start, Completed, ControlTypes, ControlTyped
<1> QED BY <1>1, <1>2, <1>3, <1>4 DEF Inv
THEOREM RetainIntentPreserves == \A w \in Writers : Inv /\ RetainIntent(w) => Inv'
<1>1. \A w \in Writers : Inv /\ RetainIntent(w) => TypeOK'
  BY SMT DEF Inv, TypeOK, StateShape, HistoryTypes, InventoryTypes, StagingTypes, ControlTypes, RetainIntent, ControlTyped, Phases, Results, StagingTyped, Inputs, Releases, OpOf, Ops
<1>2. \A w \in Writers : Inv /\ RetainIntent(w) => HistorySound'
  BY SMT DEF Inv, TypeOK, StateShape, HistorySound, RetainIntent
<1>3. \A w \in Writers : Inv /\ RetainIntent(w) => StageSound'
  <2>1. \A w \in Writers : Inv /\ RetainIntent(w) =>
    \A o \in Ops : m.intent[o] # 0 => m'.intent[o]=m.intent[o]
    BY SMT, InputShape DEF InputShape, Inv, TypeOK, StateShape,
      StagingTypes, StagingTyped, RetainIntent
  <2>2. \A w \in Writers : Inv /\ RetainIntent(w) =>
    \A o \in Ops : m'.intent[o] # 0 => OpOf(m'.intent[o])=o
    BY SMT, InputShape DEF InputShape, Inv, TypeOK, StateShape,
      StagingTypes, StagingTyped, StageSound, StageValid, RetainIntent
  <2>3. QED BY SMT, <2>1, <2>2, InputShape
    DEF InputShape, Inv, TypeOK, StateShape, StageSound, StageValid,
      RetainIntent, StagingTypes, StagingTyped, Inputs
<1>4. \A w \in Writers : Inv /\ RetainIntent(w) => CompletionSound'
  BY SMT DEF Inv, TypeOK, StateShape, CompletionSound, RetainIntent
<1> QED BY <1>1, <1>2, <1>3, <1>4 DEF Inv
THEOREM StagePreserves == \A w \in Writers : Inv /\ Stage(w) => Inv'
<1>1. \A w \in Writers : Inv /\ Stage(w) => TypeOK'
  <2>1. \A w \in Writers : Inv /\ Stage(w) => StateShape' /\ HistoryTypes'
    BY SMT DEF Inv, TypeOK, StateShape, HistoryTypes, Stage
  <2>2. \A w \in Writers : Inv /\ Stage(w) => InventoryTypes'
    <3>1. ASSUME NEW w \in Writers, Inv, Stage(w) PROVE InventoryTypes'
      <4> USE Inv, Stage(w)
      <4>1. m.request[w] \in Inputs /\ InventoryOf(m.request[w]) \in {0,1}
        BY SMT, InputShape DEF InputShape, Stage
      <4>2. InventoryTyped(m.inventory,m.original,m.byteRoots,m.accounts)
        BY DEF Inv, TypeOK, InventoryTypes
      <4>3. m'.inventory=m.inventory /\ m'.original=m.original /\
        m'.byteRoots=m.byteRoots \cup {InventoryOf(m.request[w])} /\
        m'.accounts=[m.accounts EXCEPT ![m.request[w]]=Accounting(InventoryOf(m.request[w]))]
        BY SMT, <4>1, <4>2 DEF Inv, TypeOK, StateShape, InventoryTyped, Stage
      <4>4. DOMAIN Accounting(InventoryOf(m.request[w]))=Files /\
        Accounting(InventoryOf(m.request[w])) =
          [f \in Files |-> Accounting(InventoryOf(m.request[w]))[f]]
        BY SMT DEF Accounting
      <4>5. \A f \in Files : Accounting(InventoryOf(m.request[w]))[f] \in
        [bytes : Files \X {0,1},
         disposition : {"imported","preserved","quarantined","rejected"},
         reason : {"unapproved-source-only","opaque-byte-preservation",
                   "untrusted-log-not-evidence","legacy-accepted-is-not-approval"}]
        BY <4>1, AccountingTyped DEF AccountingTyped
      <4>6. DOMAIN m'.accounts=Inputs /\
        m'.accounts=[i \in Inputs |-> m'.accounts[i]]
        BY SMT, <4>1, <4>2, <4>3 DEF InventoryTyped
      <4>7. \A i \in Inputs :
        /\ DOMAIN m'.accounts[i] \subseteq Files
        /\ m'.accounts[i]=[f \in DOMAIN m'.accounts[i] |-> m'.accounts[i][f]]
        /\ (\A f \in DOMAIN m'.accounts[i] : m'.accounts[i][f] \in
             [bytes : Files \X {0,1},
              disposition : {"imported","preserved","quarantined","rejected"},
              reason : {"unapproved-source-only","opaque-byte-preservation",
                        "untrusted-log-not-evidence","legacy-accepted-is-not-approval"}])
        BY SMT, <4>1, <4>2, <4>3, <4>4, <4>5 DEF InventoryTyped
      <4>8. QED BY SMT, <4>1, <4>2, <4>3, <4>6, <4>7
        DEF InventoryTypes, InventoryTyped
    <3>2. QED BY <3>1
  <2>3. \A w \in Writers : Inv /\ Stage(w) => StagingTypes'
    BY SMT DEF Inv, TypeOK, StateShape, StagingTypes, StagingTyped, Stage
  <2>4. \A w \in Writers : Inv /\ Stage(w) => ControlTypes'
    BY SMT DEF Inv, TypeOK, StateShape, ControlTypes, ControlTyped, Stage, Phases
  <2>5. QED BY <2>1, <2>2, <2>3, <2>4 DEF TypeOK
<1>2. \A w \in Writers : Inv /\ Stage(w) => HistorySound'
  BY SMT DEF Inv, TypeOK, StateShape, HistorySound, Stage
<1>3. \A w \in Writers : Inv /\ Stage(w) => StageSound'
  BY SMT DEF Inv, TypeOK, StateShape, StageSound, Stage, StageValid, StageReady, CompleteAccounting, BoundIntent, StagingTypes, StagingTyped, InventoryTypes, InventoryTyped, Valid, Authority
<1>4. \A w \in Writers : Inv /\ Stage(w) => CompletionSound'
  BY SMT DEF Inv, TypeOK, StateShape, CompletionSound, Stage
<1> QED BY <1>1, <1>2, <1>3, <1>4 DEF Inv
THEOREM RestoreOriginalPreserves == \A w \in Writers : Inv /\ RestoreOriginal(w) => Inv'
<1>1. \A w \in Writers : Inv /\ RestoreOriginal(w) => TypeOK'
  BY SMT DEF Inv, TypeOK, StateShape, HistoryTypes, InventoryTypes, StagingTypes, ControlTypes, RestoreOriginal, StagingTyped
<1>2. \A w \in Writers : Inv /\ RestoreOriginal(w) => HistorySound'
  BY SMT DEF Inv, TypeOK, StateShape, HistorySound, RestoreOriginal
<1>3. \A w \in Writers : Inv /\ RestoreOriginal(w) => StageSound'
  BY SMT DEF Inv, TypeOK, StateShape, StageSound, RestoreOriginal, StageValid, StageReady, CompleteAccounting, BoundIntent, StagingTypes, StagingTyped
<1>4. \A w \in Writers : Inv /\ RestoreOriginal(w) => CompletionSound'
  BY SMT DEF Inv, TypeOK, StateShape, CompletionSound, RestoreOriginal
<1> QED BY <1>1, <1>2, <1>3, <1>4 DEF Inv
THEOREM VerifyStagePreserves == \A w \in Writers : Inv /\ VerifyStage(w) => Inv'
<1>1. \A w \in Writers : Inv /\ VerifyStage(w) => TypeOK'
  BY SMT DEF Inv, TypeOK, StateShape, HistoryTypes, InventoryTypes, StagingTypes, ControlTypes, VerifyStage, ControlTyped, Phases, Results, StagingTyped
<1>2. \A w \in Writers : Inv /\ VerifyStage(w) => HistorySound'
  BY SMT DEF Inv, TypeOK, StateShape, HistorySound, VerifyStage
<1>3. \A w \in Writers : Inv /\ VerifyStage(w) => StageSound'
  BY SMT DEF Inv, TypeOK, StateShape, StageSound, VerifyStage, StageValid, StageReady, CompleteAccounting, BoundIntent, StagingTypes, StagingTyped
<1>4. \A w \in Writers : Inv /\ VerifyStage(w) => CompletionSound'
  BY SMT DEF Inv, TypeOK, StateShape, CompletionSound, VerifyStage
<1> QED BY <1>1, <1>2, <1>3, <1>4 DEF Inv
THEOREM ApplyPreserves == \A w \in Writers : Inv /\ Apply(w) => Inv'
<1>1. \A w \in Writers : Inv /\ Apply(w) => TypeOK'
  <2>1. \A w \in Writers : Inv /\ Apply(w) =>
    Candidate(m.request[w])=Record(m.request[w]) /\ m.request[w] \in Releases
    BY SMT, CandidatePreservesHistory, InputShape
      DEF CandidatePreservesHistory, InputShape, Apply
  <2>2. \A w \in Writers : Inv /\ Apply(w) => StateShape' /\ InventoryTypes'
    BY SMT DEF Inv, TypeOK, StateShape, InventoryTypes, Apply
  <2>3. \A w \in Writers : Inv /\ Apply(w) => HistoryTypes'
    <3>1. ASSUME NEW w \in Writers, Inv, Apply(w) PROVE HistoryTypes'
      <4> USE Inv, Apply(w)
      <4>1. m.request[w] \in Releases /\ Candidate(m.request[w])=Record(m.request[w])
        BY <2>1
      <4>2. HistoryTyped(m.head,m.retained,m.records)
        BY DEF Inv, TypeOK, HistoryTypes
      <4>3. m'.head=m.request[w] /\ m'.retained=m.retained \cup {m.request[w]} /\
        m'.records=[m.records EXCEPT ![m.request[w]]=Record(m.request[w])]
        BY SMT, <4>1 DEF Inv, TypeOK, StateShape, HistoryTypes, HistoryTyped, Apply
      <4>4. m.head \in Releases
        BY <4>2 DEF HistoryTyped
      <4>5. HistoryTyped(m.request[w],m.retained \cup {m.request[w]},
        [m.records EXCEPT ![m.request[w]]=Record(m.request[w])])
        BY <4>1, <4>2, <4>4, HistoryRecordReplacementTyped
          DEF HistoryRecordReplacementTyped
      <4>6. QED BY <4>3, <4>5 DEF HistoryTypes
    <3>2. QED BY <3>1
  <2>4. \A w \in Writers : Inv /\ Apply(w) => StagingTypes' /\ ControlTypes'
    BY SMT DEF Inv, TypeOK, StateShape, StagingTypes, StagingTyped,
      ControlTypes, ControlTyped, Apply, Phases
  <2>5. QED BY <2>2, <2>3, <2>4 DEF TypeOK
<1>2. \A w \in Writers : Inv /\ Apply(w) => HistorySound'
  <2>1. ASSUME NEW w \in Writers, Inv, Apply(w) PROVE HistorySound'
    <3>1. m.request[w] \in Inputs /\ m.request[w] \notin m.retained /\
      m.head=BaseOf(m.request[w]) /\ Candidate(m.request[w])=Record(m.request[w])
      BY SMT, <2>1, CandidatePreservesHistory DEF CandidatePreservesHistory, Apply
    <3>2. m'.head=m.request[w] /\ m'.retained=m.retained \cup {m.request[w]} /\
      m'.applied=m.applied \cup {m.request[w]} /\
      m'.records=[m.records EXCEPT ![m.request[w]]=Record(m.request[w])]
      BY SMT, <2>1, <3>1
        DEF Inv, TypeOK, StateShape, HistoryTypes, HistoryTyped, Apply
    <3>3. 0 \in m'.retained /\ m'.head \in m'.retained /\
      m'.applied=m'.retained \ {0}
      BY SMT, <2>1, <3>1, <3>2 DEF Inv, HistorySound, HistoryValid, Inputs
    <3>4. \A i \in m'.retained : m'.records[i]=Record(i)
      BY SMT, <2>1, <3>1, <3>2
        DEF Inv, TypeOK, HistoryTypes, HistoryTyped, HistorySound, HistoryValid, Inputs, Releases
    <3>5. m'.retained=Members(Chain(m'.head))
      BY SMT, <2>1, <3>1, <3>2, ChainExtension
        DEF ChainExtension, Inv, HistorySound, HistoryValid
    <3>6. \A i \in m'.applied : SchemaOf(i)=3 /\ BaseOf(i) \in m'.retained
      BY SMT, <2>1, <3>1, <3>2
        DEF Inv, HistorySound, HistoryValid, Apply, Valid, Authority
    <3>7. QED BY <3>3, <3>4, <3>5, <3>6 DEF HistorySound, HistoryValid
  <2>2. QED BY <2>1
<1>3. \A w \in Writers : Inv /\ Apply(w) => StageSound'
  BY SMT DEF Inv, TypeOK, StateShape, StageSound, Apply, StageValid, StageReady, CompleteAccounting, BoundIntent, StagingTypes, StagingTyped
<1>4. \A w \in Writers : Inv /\ Apply(w) => CompletionSound'
  BY SMT DEF Inv, TypeOK, StateShape, CompletionSound, Apply, Completed, ControlTypes, ControlTyped, HistoryTypes, HistoryTyped, HistorySound, HistoryValid, Inputs
<1> QED BY <1>1, <1>2, <1>3, <1>4 DEF Inv
THEOREM RecordReceiptPreserves == \A w \in Writers : Inv /\ RecordReceipt(w) => Inv'
<1>1. \A w \in Writers : Inv /\ RecordReceipt(w) => TypeOK'
  BY SMT DEF Inv, TypeOK, StateShape, HistoryTypes, InventoryTypes, StagingTypes, ControlTypes, RecordReceipt, ControlTyped, Phases, Results, StagingTyped
<1>2. \A w \in Writers : Inv /\ RecordReceipt(w) => HistorySound'
  BY SMT DEF Inv, TypeOK, StateShape, HistorySound, RecordReceipt
<1>3. \A w \in Writers : Inv /\ RecordReceipt(w) => StageSound'
  BY SMT DEF Inv, TypeOK, StateShape, StageSound, RecordReceipt, StageValid, StageReady, CompleteAccounting, BoundIntent, StagingTypes, StagingTyped
<1>4. \A w \in Writers : Inv /\ RecordReceipt(w) => CompletionSound'
  BY SMT DEF Inv, TypeOK, StateShape, CompletionSound, RecordReceipt, Completed, ControlTypes, ControlTyped
<1> QED BY <1>1, <1>2, <1>3, <1>4 DEF Inv
THEOREM ReplayPreserves == \A w \in Writers : Inv /\ Replay(w) => Inv'
<1>1. \A w \in Writers : Inv /\ Replay(w) => TypeOK'
  BY SMT DEF Inv, TypeOK, StateShape, HistoryTypes, InventoryTypes, StagingTypes, ControlTypes, Replay, ControlTyped, Phases, Results, StagingTyped
<1>2. \A w \in Writers : Inv /\ Replay(w) => HistorySound'
  BY SMT DEF Inv, TypeOK, StateShape, HistorySound, Replay
<1>3. \A w \in Writers : Inv /\ Replay(w) => StageSound'
  BY SMT DEF Inv, TypeOK, StateShape, StageSound, Replay, StageValid, StageReady, CompleteAccounting, BoundIntent, StagingTypes, StagingTyped
<1>4. \A w \in Writers : Inv /\ Replay(w) => CompletionSound'
  BY SMT DEF Inv, TypeOK, StateShape, CompletionSound, Replay, Completed, ControlTypes, ControlTyped
<1> QED BY <1>1, <1>2, <1>3, <1>4 DEF Inv
THEOREM RecoverPreserves == \A w \in Writers : Inv /\ Recover(w) => Inv'
<1>1. \A w \in Writers : Inv /\ Recover(w) => TypeOK'
  BY SMT DEF Inv, TypeOK, StateShape, HistoryTypes, InventoryTypes, StagingTypes, ControlTypes, Recover, ControlTyped, Phases, Results
<1>2. \A w \in Writers : Inv /\ Recover(w) => HistorySound'
  BY SMT DEF Inv, TypeOK, StateShape, HistorySound, Recover
<1>3. \A w \in Writers : Inv /\ Recover(w) => StageSound'
  BY SMT DEF Inv, TypeOK, StateShape, StageSound, Recover
<1>4. \A w \in Writers : Inv /\ Recover(w) => CompletionSound'
  BY SMT DEF Inv, TypeOK, StateShape, CompletionSound, Recover
<1> QED BY <1>1, <1>2, <1>3, <1>4 DEF Inv
THEOREM ConflictPreserves == \A w \in Writers : Inv /\ Conflict(w) => Inv'
<1>1. \A w \in Writers : Inv /\ Conflict(w) => TypeOK'
  BY SMT DEF Inv, TypeOK, StateShape, HistoryTypes, InventoryTypes, StagingTypes, ControlTypes, Conflict, ControlTyped, Phases, Results
<1>2. \A w \in Writers : Inv /\ Conflict(w) => HistorySound'
  BY SMT DEF Inv, TypeOK, StateShape, HistorySound, Conflict
<1>3. \A w \in Writers : Inv /\ Conflict(w) => StageSound'
  BY SMT DEF Inv, TypeOK, StateShape, StageSound, Conflict
<1>4. \A w \in Writers : Inv /\ Conflict(w) => CompletionSound'
  BY SMT DEF Inv, TypeOK, StateShape, CompletionSound, Conflict, Completed, ControlTypes, ControlTyped
<1> QED BY <1>1, <1>2, <1>3, <1>4 DEF Inv
THEOREM RefusePreserves == \A w \in Writers : Inv /\ Refuse(w) => Inv'
<1>1. \A w \in Writers : Inv /\ Refuse(w) => TypeOK'
  BY SMT DEF Inv, TypeOK, StateShape, HistoryTypes, InventoryTypes, StagingTypes, ControlTypes, Refuse, ControlTyped, Phases, Results
<1>2. \A w \in Writers : Inv /\ Refuse(w) => HistorySound'
  BY SMT DEF Inv, TypeOK, StateShape, HistorySound, Refuse
<1>3. \A w \in Writers : Inv /\ Refuse(w) => StageSound'
  BY SMT DEF Inv, TypeOK, StateShape, StageSound, Refuse
<1>4. \A w \in Writers : Inv /\ Refuse(w) => CompletionSound'
  BY SMT DEF Inv, TypeOK, StateShape, CompletionSound, Refuse, Completed, ControlTypes, ControlTyped
<1> QED BY <1>1, <1>2, <1>3, <1>4 DEF Inv
THEOREM FailIOPreserves == \A w \in Writers : Inv /\ FailIO(w) => Inv'
<1>1. \A w \in Writers : Inv /\ FailIO(w) => TypeOK'
  BY SMT DEF Inv, TypeOK, StateShape, HistoryTypes, InventoryTypes, StagingTypes, ControlTypes, FailIO, ControlTyped, Phases, Results
<1>2. \A w \in Writers : Inv /\ FailIO(w) => HistorySound'
  BY SMT DEF Inv, TypeOK, StateShape, HistorySound, FailIO
<1>3. \A w \in Writers : Inv /\ FailIO(w) => StageSound'
  BY SMT DEF Inv, TypeOK, StateShape, StageSound, FailIO
<1>4. \A w \in Writers : Inv /\ FailIO(w) => CompletionSound'
  BY SMT DEF Inv, TypeOK, StateShape, CompletionSound, FailIO, Completed, ControlTypes, ControlTyped
<1> QED BY <1>1, <1>2, <1>3, <1>4 DEF Inv
THEOREM CrashPreserves == \A w \in Writers : Inv /\ Crash(w) => Inv'
<1>1. \A w \in Writers : Inv /\ Crash(w) => TypeOK'
  BY SMT DEF Inv, TypeOK, StateShape, HistoryTypes, InventoryTypes, StagingTypes, ControlTypes, Crash, ControlTyped, Phases, Results
<1>2. \A w \in Writers : Inv /\ Crash(w) => HistorySound'
  BY SMT DEF Inv, TypeOK, StateShape, HistorySound, Crash
<1>3. \A w \in Writers : Inv /\ Crash(w) => StageSound'
  BY SMT DEF Inv, TypeOK, StateShape, StageSound, Crash
<1>4. \A w \in Writers : Inv /\ Crash(w) => CompletionSound'
  BY SMT DEF Inv, TypeOK, StateShape, CompletionSound, Crash, Completed, ControlTypes, ControlTyped
<1> QED BY <1>1, <1>2, <1>3, <1>4 DEF Inv
THEOREM RestartPreserves == \A w \in Writers : Inv /\ Restart(w) => Inv'
<1>1. \A w \in Writers : Inv /\ Restart(w) => TypeOK'
  BY SMT DEF Inv, TypeOK, StateShape, HistoryTypes, InventoryTypes, StagingTypes, ControlTypes, Restart, ControlTyped, Phases, Results, Inputs, Releases, OpOf, Ops
<1>2. \A w \in Writers : Inv /\ Restart(w) => HistorySound'
  BY SMT DEF Inv, TypeOK, StateShape, HistorySound, Restart
<1>3. \A w \in Writers : Inv /\ Restart(w) => StageSound'
  BY SMT DEF Inv, TypeOK, StateShape, StageSound, Restart
<1>4. \A w \in Writers : Inv /\ Restart(w) => CompletionSound'
  BY SMT DEF Inv, TypeOK, StateShape, CompletionSound, Restart, Completed, ControlTypes, ControlTyped
<1> QED BY <1>1, <1>2, <1>3, <1>4 DEF Inv
THEOREM ObserveProfilePreserves == \A f \in Profiles : Inv /\ ObserveProfile(f) => Inv'
<1>1. \A f \in Profiles : Inv /\ ObserveProfile(f) => TypeOK'
  BY SMT DEF Inv, TypeOK, StateShape, HistoryTypes, InventoryTypes, StagingTypes, ControlTypes, ObserveProfile, ControlTyped, Phases, Results
<1>2. \A f \in Profiles : Inv /\ ObserveProfile(f) => HistorySound'
  BY SMT DEF Inv, TypeOK, StateShape, HistorySound, ObserveProfile
<1>3. \A f \in Profiles : Inv /\ ObserveProfile(f) => StageSound'
  BY SMT DEF Inv, TypeOK, StateShape, StageSound, ObserveProfile
<1>4. \A f \in Profiles : Inv /\ ObserveProfile(f) => CompletionSound'
  BY SMT DEF Inv, TypeOK, StateShape, CompletionSound, ObserveProfile
<1> QED BY <1>1, <1>2, <1>3, <1>4 DEF Inv
THEOREM LegacyReplayPreserves == \A w \in Writers, b \in LegacyInputs : Inv /\ LegacyReplay(w,b) => Inv'
<1>1. \A w \in Writers, b \in LegacyInputs : Inv /\ LegacyReplay(w,b) => TypeOK'
  BY SMT DEF Inv, TypeOK, StateShape, HistoryTypes, InventoryTypes, StagingTypes, ControlTypes, LegacyReplay, ControlTyped, Phases, Results, Inputs, Releases, OpOf, Ops
<1>2. \A w \in Writers, b \in LegacyInputs : Inv /\ LegacyReplay(w,b) => HistorySound'
  BY SMT DEF Inv, TypeOK, StateShape, HistorySound, LegacyReplay
<1>3. \A w \in Writers, b \in LegacyInputs : Inv /\ LegacyReplay(w,b) => StageSound'
  BY SMT DEF Inv, TypeOK, StateShape, StageSound, LegacyReplay
<1>4. \A w \in Writers, b \in LegacyInputs : Inv /\ LegacyReplay(w,b) => CompletionSound'
  BY SMT DEF Inv, TypeOK, StateShape, CompletionSound, LegacyReplay, Completed, ControlTypes, ControlTyped
<1> QED BY <1>1, <1>2, <1>3, <1>4 DEF Inv
THEOREM LegacyReplayFrame == \A w \in Writers, b \in LegacyInputs :
  TypeOK /\ LegacyReplay(w,b) => UNCHANGED Durable
BY SMT DEF TypeOK, StateShape, LegacyReplay, Durable
(* Sequence laws are independent of record updates and validator observations.
   Exposing the typed source sequence first keeps append reasoning out of the
   complete nested state type and inventory-accounting obligation. *)
THEOREM AppendExtendsPrefix == \A s \in Seq(Releases), i \in Releases :
  Prefix(s,Append(s,i))
BY SMT DEF Prefix
THEOREM PrefixReflexive == \A s \in Seq(Releases) : Prefix(s,s)
BY SMT DEF Prefix
THEOREM CurrentHistoryTyped == Inv => m.records[m.head].history \in Seq(Releases)
<1>1. Inv => m.head \in Releases /\ m.records[m.head]=Record(m.head)
  BY SMT DEF Inv, TypeOK, HistoryTypes, HistoryTyped, HistorySound, HistoryValid
<1>2. QED BY SMT, <1>1, ChainShape DEF ChainShape, Record
THEOREM ApplyHistoryMonotone == \A w \in Writers : Inv /\ Apply(w) =>
  /\ m.retained \subseteq m'.retained /\ m.applied \subseteq m'.applied
  /\ Prefix(m.records[m.head].history,m'.records[m'.head].history)
  /\ m.records[m.head].corrections \subseteq m'.records[m'.head].corrections
<1>1. ASSUME NEW w \in Writers, Inv, Apply(w)
      PROVE /\ m.retained \subseteq m'.retained /\ m.applied \subseteq m'.applied
            /\ Prefix(m.records[m.head].history,m'.records[m'.head].history)
            /\ m.records[m.head].corrections \subseteq m'.records[m'.head].corrections
  <2>1. m.request[w] \in Releases /\
    m.records[m.head].history \in Seq(Releases)
    BY SMT, <1>1, InputShape, CurrentHistoryTyped
      DEF InputShape, CurrentHistoryTyped, Apply
  <2>2. m'.head=m.request[w] /\ m'.records[m'.head]=Candidate(m.request[w])
    BY SMT, <1>1, <2>1
      DEF Inv, TypeOK, StateShape, HistoryTypes, HistoryTyped, Apply
  <2>3. m.retained \subseteq m'.retained /\ m.applied \subseteq m'.applied
    BY SMT, <1>1 DEF Inv, TypeOK, StateShape, Apply
  <2>4. Prefix(m.records[m.head].history,m'.records[m'.head].history)
    BY SMT, <2>1, <2>2, AppendExtendsPrefix DEF AppendExtendsPrefix, Candidate
  <2>5. m.records[m.head].corrections \subseteq m'.records[m'.head].corrections
    BY SMT, <2>2 DEF Candidate
  <2>6. QED BY <2>3, <2>4, <2>5
<1>2. QED BY <1>1
THEOREM NonApplyHistoryFrame == Inv /\ Next =>
  (\E w \in Writers : Apply(w)) \/ UNCHANGED <<m.head,m.retained,m.records,m.applied>>
BY SMT DEF Inv, TypeOK, StateShape, Next, CoreNext, DryRun, Stutter,
  HistoricalRead, LegacyReplay, InventoryChange, Observe, Start, RetainIntent,
  Stage, RestoreOriginal, VerifyStage, RecordReceipt, Replay, Recover,
  Conflict, Refuse, FailIO, Crash, Restart
THEOREM NextHistoryMonotone == Inv /\ Next =>
  /\ m.retained \subseteq m'.retained /\ m.applied \subseteq m'.applied
  /\ Prefix(m.records[m.head].history,m'.records[m'.head].history)
  /\ m.records[m.head].corrections \subseteq m'.records[m'.head].corrections
<1>1. ASSUME Inv, Next
      PROVE /\ m.retained \subseteq m'.retained /\ m.applied \subseteq m'.applied
            /\ Prefix(m.records[m.head].history,m'.records[m'.head].history)
            /\ m.records[m.head].corrections \subseteq m'.records[m'.head].corrections
  <2>1. CASE \E w \in Writers : Apply(w)
    BY SMT, <1>1, <2>1, ApplyHistoryMonotone DEF ApplyHistoryMonotone
  <2>2. CASE UNCHANGED <<m.head,m.retained,m.records,m.applied>>
    BY SMT, <1>1, <2>2, CurrentHistoryTyped, PrefixReflexive
      DEF CurrentHistoryTyped, PrefixReflexive
  <2>3. QED BY <1>1, <2>1, <2>2, NonApplyHistoryFrame DEF NonApplyHistoryFrame
<1>2. QED BY <1>1
THEOREM InvImpliesSafety == Inv => Safety
<1>1. Inv => \A i \in m.applied :
    /\ CompleteAccounting(i) /\ BoundIntent(i) /\ Input(i).schema=3
    /\ m.records[i].history=Chain(i) /\ m.records[i].corrections=Corrections(i)
    /\ m.records[i].contexts=Family(i) /\ m.records[i].selected=Selected(i)
  BY SMT DEF Inv, HistorySound, HistoryValid, StageSound, StageValid,
    CompleteAccounting, BoundIntent, Record, Input
<1>2. Inv => m.original=Inventory(0)
  BY SMT DEF Inv, TypeOK, InventoryTypes, InventoryTyped
<1>3. Inv => /\ 0 \in m.byteRoots /\ m.records[0]=Record(0)
             /\ m.receipts \subseteq m.applied /\ CompletionSound
  BY SMT DEF Inv, HistorySound, HistoryValid, StageSound, StageValid
<1>4. Inv => \A i \in m.applied : m.records[i].corrections # {} =>
       "old-supported" \notin Supported(i)
  BY SMT DEF Supported
<1> QED BY <1>1, <1>2, <1>3, <1>4 DEF Safety
THEOREM NextPreserves == Inv /\ Next => Inv'
BY HistoricalReadPreserves, InventoryChangePreserves, ObservePreserves, StartPreserves, RetainIntentPreserves, StagePreserves, RestoreOriginalPreserves, VerifyStagePreserves, ApplyPreserves, RecordReceiptPreserves, ReplayPreserves, RecoverPreserves, ConflictPreserves, RefusePreserves, FailIOPreserves, CrashPreserves, RestartPreserves, LegacyReplayPreserves, DryRunPreserves, StutterPreserves
  DEF HistoricalReadPreserves, InventoryChangePreserves, ObservePreserves, StartPreserves, RetainIntentPreserves, StagePreserves, RestoreOriginalPreserves, VerifyStagePreserves, ApplyPreserves, RecordReceiptPreserves, ReplayPreserves, RecoverPreserves, ConflictPreserves, RefusePreserves, FailIOPreserves, CrashPreserves, RestartPreserves, LegacyReplayPreserves, DryRunPreserves, StutterPreserves, Next, CoreNext
THEOREM FiniteNextPreserves == Inv /\ FiniteNext => Inv'
BY HistoricalReadPreserves, InventoryChangePreserves, StartPreserves, RetainIntentPreserves, StagePreserves, RestoreOriginalPreserves, VerifyStagePreserves, ApplyPreserves, RecordReceiptPreserves, ReplayPreserves, RecoverPreserves, ConflictPreserves, RefusePreserves, FailIOPreserves, CrashPreserves, RestartPreserves, ObserveProfilePreserves, LegacyReplayPreserves, DryRunPreserves, StutterPreserves
  DEF HistoricalReadPreserves, InventoryChangePreserves, StartPreserves, RetainIntentPreserves, StagePreserves, RestoreOriginalPreserves, VerifyStagePreserves, ApplyPreserves, RecordReceiptPreserves, ReplayPreserves, RecoverPreserves, ConflictPreserves, RefusePreserves, FailIOPreserves, CrashPreserves, RestartPreserves, ObserveProfilePreserves, LegacyReplayPreserves, DryRunPreserves, StutterPreserves, FiniteNext, CoreNext
THEOREM DryRunAccountsWithoutAdmission == Inv /\ DryRun =>
  /\ DOMAIN DryRunReport.accounting=Files /\ DryRunReport.approvedKnowledge={}
  /\ UNCHANGED Durable
BY SMT DEF DryRun, DryRunReport, Accounting, Durable
THEOREM CurrentSelectionDoesNotResurrectSupport == Inv =>
  (m.records[m.head].corrections # {} => "old-supported" \notin Supported(m.head))
BY SMT DEF Supported
=============================================================================
