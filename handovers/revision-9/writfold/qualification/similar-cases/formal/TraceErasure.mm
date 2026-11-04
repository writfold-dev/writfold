$( Structural induction on finite traces of arbitrary length.
   A map sends each trace to a state transformer. live evaluates operations;
   erased folds only separately checked witnesses. A read can contain any
   observation or arbitrary ranked payload. The atom holds a transformer,
   not one initial state. No finiteness axiom on states, witnesses or ranks.
   Eq hypotheses in 'if' are equational sequents. The only induction rule
   below is ordinary constructor induction with fresh eigenvariables. $)
$c tr pay wit map fn ( ) nil read put live erased at unit extend =
    eq if then all $.
$v t r w F G X Y A B C D $.
vt $f tr t $. vr $f pay r $. vw $f wit w $.
vF $f map F $. vG $f map G $.
vX $f fn X $. vY $f fn Y $. vA $f fn A $. vB $f fn B $. vC $f fn C $. vD $f fn D $.
Nil $a tr nil $. Read $a tr ( read t r ) $. Put $a tr ( put t w ) $.
Live $a map live $. Erased $a map erased $. At $a fn ( at F t ) $.
Unit $a fn unit $. Extend $a fn ( extend X w ) $.

$( Six defining equations for the two interpretations. Left is the executable
   fold. Right is erase-then-fold. Read/peek are grouped as read. $)
live-nil $a eq ( at live nil ) = unit $.
erased-nil $a eq ( at erased nil ) = unit $.
live-read $a eq ( at live ( read t r ) ) = ( at live t ) $.
erased-read $a eq ( at erased ( read t r ) ) = ( at erased t ) $.
live-put $a eq ( at live ( put t w ) ) = ( extend ( at live t ) w ) $.
erased-put $a eq ( at erased ( put t w ) ) = ( extend ( at erased t ) w ) $.

$( Standard equality logic. $)
${ ba $e eq A = C $. bb $e eq B = C $.
  bridge $a eq A = B $. $}
hyp $a if X = Y then X = Y $.
${ congr-h $e if X = Y then A = B $.
  congr $a if X = Y then ( extend A w ) = ( extend B w ) $. $}
${ sl $e eq A = C $. sm $e if X = Y then C = D $. sr $e eq B = D $.
  rewrite $a if X = Y then A = B $. $}

$( General structural induction rule: F,G stand for fixed maps, independent
   of t,r,w. Each step premise is for arbitrary fresh tail and payload. $)
${ $d F t $. $d F r $. $d F w $. $d G t $. $d G r $. $d G w $.
  i0 $e eq ( at F nil ) = ( at G nil ) $.
  ir $e if ( at F t ) = ( at G t ) then ( at F ( read t r ) ) = ( at G ( read t r ) ) $.
  iw $e if ( at F t ) = ( at G t ) then ( at F ( put t w ) ) = ( at G ( put t w ) ) $.
  induction $a all F = G $. $}

base $p eq ( at live nil ) = ( at erased nil ) $=
 Nil Live At Nil Erased At Unit live-nil erased-nil bridge $.

read-step $p if ( at live t ) = ( at erased t ) then
  ( at live ( read t r ) ) = ( at erased ( read t r ) ) $=
 vt Live At vt Erased At vt vr Read Live At vt vr Read Erased At
 vt Live At vt Erased At vt vr live-read vt Live At vt Erased At hyp
 vt vr erased-read rewrite $.

put-step $p if ( at live t ) = ( at erased t ) then
  ( at live ( put t w ) ) = ( at erased ( put t w ) ) $=
 vt Live At vt Erased At vt vw Put Live At vt vw Put Erased At
 vw vt Live At Extend vw vt Erased At Extend
 vt vw live-put
 vw vt Live At vt Erased At vt Live At vt Erased At vt Live At vt Erased At hyp congr
 vt vw erased-put rewrite $.

trace-erasure $p all live = erased $=
 vt vr vw Live Erased base vt vr read-step vt vw put-step induction $.
