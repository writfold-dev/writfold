---- MODULE Invocation_TTrace_1795693440 ----
EXTENDS Sequences, TLCExt, Invocation, Toolbox, Naturals, TLC

_expression ==
    LET Invocation_TEExpression == INSTANCE Invocation_TEExpression
    IN Invocation_TEExpression!expression
----

_trace ==
    LET Invocation_TETrace == INSTANCE Invocation_TETrace
    IN Invocation_TETrace!trace
----

_inv ==
    ~(
        TLCGet("level") = Len(_TETrace)
        /\
        bad = (FALSE)
        /\
        last = ("deliver")
        /\
        pending = ([a |-> {}, b |-> {}])
        /\
        prep = ([a |-> [i1 |-> [step |-> "s1", digest |-> 0], i2 |-> [step |-> "s1", digest |-> 0]], b |-> [i1 |-> [step |-> "none", digest |-> 0], i2 |-> [step |-> "none", digest |-> 0]]])
        /\
        checked = ([a |-> {[inv |-> "i1", step |-> "s1", payload |-> "p"]}, b |-> {}])
        /\
        crashes = (0)
        /\
        done = ([a |-> {[inv |-> "i1", step |-> "s1", payload |-> "p"]}, b |-> {}])
        /\
        sent = (2)
        /\
        closure = ([a |-> 0, b |-> 0])
        /\
        live = ([a |-> {[inv |-> "i1", step |-> "s1", payload |-> "p"], [inv |-> "i1", step |-> "s1", payload |-> "q"]}, b |-> {}])
        /\
        gone = ([a |-> {}, b |-> {}])
    )
----

_init ==
    /\ bad = _TETrace[1].bad
    /\ done = _TETrace[1].done
    /\ prep = _TETrace[1].prep
    /\ gone = _TETrace[1].gone
    /\ checked = _TETrace[1].checked
    /\ pending = _TETrace[1].pending
    /\ last = _TETrace[1].last
    /\ sent = _TETrace[1].sent
    /\ crashes = _TETrace[1].crashes
    /\ closure = _TETrace[1].closure
    /\ live = _TETrace[1].live
----

_next ==
    /\ \E i,j \in DOMAIN _TETrace:
        /\ \/ /\ j = i + 1
              /\ i = TLCGet("level")
        /\ bad  = _TETrace[i].bad
        /\ bad' = _TETrace[j].bad
        /\ done  = _TETrace[i].done
        /\ done' = _TETrace[j].done
        /\ prep  = _TETrace[i].prep
        /\ prep' = _TETrace[j].prep
        /\ gone  = _TETrace[i].gone
        /\ gone' = _TETrace[j].gone
        /\ checked  = _TETrace[i].checked
        /\ checked' = _TETrace[j].checked
        /\ pending  = _TETrace[i].pending
        /\ pending' = _TETrace[j].pending
        /\ last  = _TETrace[i].last
        /\ last' = _TETrace[j].last
        /\ sent  = _TETrace[i].sent
        /\ sent' = _TETrace[j].sent
        /\ crashes  = _TETrace[i].crashes
        /\ crashes' = _TETrace[j].crashes
        /\ closure  = _TETrace[i].closure
        /\ closure' = _TETrace[j].closure
        /\ live  = _TETrace[i].live
        /\ live' = _TETrace[j].live

\* Uncomment the ASSUME below to write the states of the error trace
\* to the given file in Json format. Note that you can pass any tuple
\* to `JsonSerialize`. For example, a sub-sequence of _TETrace.
    \* ASSUME
    \*     LET J == INSTANCE Json
    \*         IN J!JsonSerialize("Invocation_TTrace_1795693440.json", _TETrace)

=============================================================================

 Note that you can extract this module `Invocation_TEExpression`
  to a dedicated file to reuse `expression` (the module in the 
  dedicated `Invocation_TEExpression.tla` file takes precedence 
  over the module `Invocation_TEExpression` below).

---- MODULE Invocation_TEExpression ----
EXTENDS Sequences, TLCExt, Invocation, Toolbox, Naturals, TLC

expression == 
    [
        \* To hide variables of the `Invocation` spec from the error trace,
        \* remove the variables below.  The trace will be written in the order
        \* of the fields of this record.
        bad |-> bad
        ,done |-> done
        ,prep |-> prep
        ,gone |-> gone
        ,checked |-> checked
        ,pending |-> pending
        ,last |-> last
        ,sent |-> sent
        ,crashes |-> crashes
        ,closure |-> closure
        ,live |-> live
        
        \* Put additional constant-, state-, and action-level expressions here:
        \* ,_stateNumber |-> _TEPosition
        \* ,_badUnchanged |-> bad = bad'
        
        \* Format the `bad` variable as Json value.
        \* ,_badJson |->
        \*     LET J == INSTANCE Json
        \*     IN J!ToJson(bad)
        
        \* Lastly, you may build expressions over arbitrary sets of states by
        \* leveraging the _TETrace operator.  For example, this is how to
        \* count the number of times a spec variable changed up to the current
        \* state in the trace.
        \* ,_badModCount |->
        \*     LET F[s \in DOMAIN _TETrace] ==
        \*         IF s = 1 THEN 0
        \*         ELSE IF _TETrace[s].bad # _TETrace[s-1].bad
        \*             THEN 1 + F[s-1] ELSE F[s-1]
        \*     IN F[_TEPosition - 1]
    ]

=============================================================================



Parsing and semantic processing can take forever if the trace below is long.
 In this case, it is advised to uncomment the module below to deserialize the
 trace from a generated binary file.

\*
\*---- MODULE Invocation_TETrace ----
\*EXTENDS IOUtils, Invocation, TLC
\*
\*trace == IODeserialize("Invocation_TTrace_1795693440.bin", TRUE)
\*
\*=============================================================================
\*

---- MODULE Invocation_TETrace ----
EXTENDS Invocation, TLC

trace == 
    <<
    ([bad |-> FALSE,last |-> "init",pending |-> [a |-> {}, b |-> {}],prep |-> [a |-> [i1 |-> [step |-> "none", digest |-> 0], i2 |-> [step |-> "none", digest |-> 0]], b |-> [i1 |-> [step |-> "none", digest |-> 0], i2 |-> [step |-> "none", digest |-> 0]]],checked |-> [a |-> {}, b |-> {}],crashes |-> 0,done |-> [a |-> {}, b |-> {}],sent |-> 0,closure |-> [a |-> 0, b |-> 0],live |-> [a |-> {}, b |-> {}],gone |-> [a |-> {}, b |-> {}]]),
    ([bad |-> FALSE,last |-> "prepare",pending |-> [a |-> {}, b |-> {}],prep |-> [a |-> [i1 |-> [step |-> "s1", digest |-> 0], i2 |-> [step |-> "none", digest |-> 0]], b |-> [i1 |-> [step |-> "none", digest |-> 0], i2 |-> [step |-> "none", digest |-> 0]]],checked |-> [a |-> {}, b |-> {}],crashes |-> 0,done |-> [a |-> {}, b |-> {}],sent |-> 0,closure |-> [a |-> 0, b |-> 0],live |-> [a |-> {}, b |-> {}],gone |-> [a |-> {}, b |-> {}]]),
    ([bad |-> FALSE,last |-> "deliver",pending |-> [a |-> {}, b |-> {}],prep |-> [a |-> [i1 |-> [step |-> "s1", digest |-> 0], i2 |-> [step |-> "none", digest |-> 0]], b |-> [i1 |-> [step |-> "none", digest |-> 0], i2 |-> [step |-> "none", digest |-> 0]]],checked |-> [a |-> {}, b |-> {}],crashes |-> 0,done |-> [a |-> {}, b |-> {}],sent |-> 1,closure |-> [a |-> 0, b |-> 0],live |-> [a |-> {[inv |-> "i1", step |-> "s1", payload |-> "p"]}, b |-> {}],gone |-> [a |-> {}, b |-> {}]]),
    ([bad |-> FALSE,last |-> "prepare",pending |-> [a |-> {}, b |-> {}],prep |-> [a |-> [i1 |-> [step |-> "s1", digest |-> 0], i2 |-> [step |-> "s1", digest |-> 0]], b |-> [i1 |-> [step |-> "none", digest |-> 0], i2 |-> [step |-> "none", digest |-> 0]]],checked |-> [a |-> {}, b |-> {}],crashes |-> 0,done |-> [a |-> {}, b |-> {}],sent |-> 1,closure |-> [a |-> 0, b |-> 0],live |-> [a |-> {[inv |-> "i1", step |-> "s1", payload |-> "p"]}, b |-> {}],gone |-> [a |-> {}, b |-> {}]]),
    ([bad |-> FALSE,last |-> "check",pending |-> [a |-> {}, b |-> {}],prep |-> [a |-> [i1 |-> [step |-> "s1", digest |-> 0], i2 |-> [step |-> "s1", digest |-> 0]], b |-> [i1 |-> [step |-> "none", digest |-> 0], i2 |-> [step |-> "none", digest |-> 0]]],checked |-> [a |-> {[inv |-> "i1", step |-> "s1", payload |-> "p"]}, b |-> {}],crashes |-> 0,done |-> [a |-> {}, b |-> {}],sent |-> 1,closure |-> [a |-> 0, b |-> 0],live |-> [a |-> {[inv |-> "i1", step |-> "s1", payload |-> "p"]}, b |-> {}],gone |-> [a |-> {}, b |-> {}]]),
    ([bad |-> FALSE,last |-> "deliver",pending |-> [a |-> {}, b |-> {}],prep |-> [a |-> [i1 |-> [step |-> "s1", digest |-> 0], i2 |-> [step |-> "s1", digest |-> 0]], b |-> [i1 |-> [step |-> "none", digest |-> 0], i2 |-> [step |-> "none", digest |-> 0]]],checked |-> [a |-> {[inv |-> "i1", step |-> "s1", payload |-> "p"]}, b |-> {}],crashes |-> 0,done |-> [a |-> {[inv |-> "i1", step |-> "s1", payload |-> "p"]}, b |-> {}],sent |-> 2,closure |-> [a |-> 0, b |-> 0],live |-> [a |-> {[inv |-> "i1", step |-> "s1", payload |-> "p"], [inv |-> "i1", step |-> "s1", payload |-> "q"]}, b |-> {}],gone |-> [a |-> {}, b |-> {}]])
    >>
----


=============================================================================

---- CONFIG Invocation_TTrace_1795693440 ----
CONSTANTS
    Single = TRUE
    Recheck = TRUE
    SameStep = TRUE
    AdvanceOnDeliver = TRUE
    RecordedStep = TRUE
    MaxCrashes = 1
    MaxDeliveries = 3

INVARIANT
    _inv

CHECK_DEADLOCK
    \* CHECK_DEADLOCK off because of PROPERTY or INVARIANT above.
    FALSE

INIT
    _init

NEXT
    _next

CONSTANT
    _TETrace <- _trace

ALIAS
    _expression
=============================================================================
\* Generated on Thu Nov 26 11:44:01 GMT 2026