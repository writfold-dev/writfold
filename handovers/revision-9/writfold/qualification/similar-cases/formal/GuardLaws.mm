$( Independent propositional certificate for local guard laws. Every variable
   ranges over arbitrary propositions. Only standard conjunction, composition
   of implications and contraposition are assumed. No Writfold axiom.
   Instantiation: a = rank proposed b; b = correct cut and binding; c = allowed
   at issuance; d = retained at issuance; e = current eligibility on peek;
   f = predecessor/witness/authority checks for an authoritative transition.
   The model assumes the engine implements these guards. $)
$c wff |- ( ) & -> ~ $.
$v a b c d e f $.
wa $f wff a $. wb $f wff b $. wc $f wff c $.
wd $f wff d $. we $f wff e $. wf $f wff f $.
an $a wff ( a & b ) $. im $a wff ( a -> b ) $. neg $a wff ~ a $.
left $a |- ( ( a & b ) -> a ) $. right $a |- ( ( a & b ) -> b ) $.
${ h1 $e |- ( a -> b ) $. h2 $e |- ( b -> c ) $.
  chain $a |- ( a -> c ) $. $}
${ hn $e |- ( a -> b ) $. contrapose $a |- ( ~ b -> ~ a ) $. $}

$( Issued = proposed AND (cut AND (visible AND retained)). $)
eligible $p |- ( ( a & ( b & ( c & d ) ) ) -> ( b & ( c & d ) ) ) $=
 wa wb wc wd an an right $.
bound $p |- ( ( a & ( b & ( c & d ) ) ) -> b ) $=
 wa wb wc wd an an an wb wc wd an an wb
 wa wb wc wd eligible wb wc wd an left chain $.
visible $p |- ( ( a & ( b & ( c & d ) ) ) -> c ) $=
 wa wb wc wd an an an wc wd an wc
 wa wb wc wd an an an wb wc wd an an wc wd an
 wa wb wc wd eligible wb wc wd an right chain
 wc wd left chain $.
$( Peek = issued AND currently eligible. If access is revoked there is no peek. $)
no-open $p |- ( ~ e -> ~ ( ( a & ( b & ( c & d ) ) ) & e ) ) $=
 wa wb wc wd an an an we an we wa wb wc wd an an an we right contrapose $.
$( Even a perfect match cannot authorize a transition without its exact gate. $)
no-apply $p |- ( ~ f -> ~ ( a & f ) ) $=
 wa wf an wf wa wf right contrapose $.
