(§3.b tf-decl-lifetime · OPEN · in Parser accept) (a `static` declaration after a task header is rejected):
* symptom: `static int c = 0;` after `task t;` is E2002
* feed `d.lifetime`
* status: OPEN
