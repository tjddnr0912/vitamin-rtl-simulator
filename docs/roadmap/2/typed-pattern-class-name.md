(§2 typed-pattern-class-name · OPEN · in Scoping / imports / block-locals) (a typed pattern over a class name shadowing a struct is accepted):
* a module-local `class s_t` shadowing a unit packed struct `s_t` leaves the struct's typedef entry under the name (a class name is registered by a prescan that runs before every typedef and never overrides one), so `s_t'{4'h3, 4'h1}` is accepted (k06 `w=31`) where verilator rejects the source (`Assignment pattern applies against non-struct/union data type: 'class{}s_t'`)
* accept-invalid
* fix: give a class declaration a registration where it is declared (`TypeInfo::bind`, §4.5.609) so the newest-registration rule refuses it
* OPEN
