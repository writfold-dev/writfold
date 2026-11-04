---- MODULE IndexPublication ----
EXTENDS Naturals
CONSTANT Epochs
ASSUME /\ Epochs # {} /\ 0 \notin Epochs
VARIABLES desired, pending, active, reader, handle, grant, pulse
vars == <<desired, pending, active, reader, handle, grant, pulse>>

Init == /\ desired \in Epochs /\ pending = 0 /\ active = 0 /\ reader = 0
        /\ handle = 0 /\ grant = TRUE /\ pulse = FALSE
Target == /\ desired' \in Epochs /\ UNCHANGED <<pending,active,reader,handle,grant,pulse>>
Build == /\ pending' = desired /\ UNCHANGED <<desired,active,reader,handle,grant,pulse>>
Publish == /\ pending # 0 /\ pending = desired
           /\ active' = pending /\ pending' = 0
           /\ UNCHANGED <<desired,reader,handle,grant,pulse>>
Attach == /\ reader' = active /\ UNCHANGED <<desired,pending,active,handle,grant,pulse>>
Toggle == /\ grant' = ~grant /\ UNCHANGED <<desired,pending,active,reader,handle,pulse>>
Issue == /\ reader # 0 /\ grant /\ handle' = reader
         /\ UNCHANGED <<desired,pending,active,reader,grant,pulse>>
Open == /\ reader # 0 /\ reader = handle /\ grant /\ pulse' = ~pulse
        /\ UNCHANGED <<desired,pending,active,reader,handle,grant>>
Next == Target \/ Build \/ Publish \/ Attach \/ Toggle \/ Issue \/ Open
Spec == Init /\ [][Next]_vars
TypeOK == /\ desired \in Epochs /\ grant \in BOOLEAN /\ pulse \in BOOLEAN
          /\ {pending,active,reader,handle} \subseteq Epochs \cup {0}
FreshPublish == [][(active' # active => active' = desired)]_vars
CheckedOpen == [][(pulse' # pulse => (reader # 0 /\ handle = reader /\ grant))]_vars
EligibleIssue == [][(handle' # handle => (handle' = reader /\ reader # 0 /\ grant))]_vars
====
