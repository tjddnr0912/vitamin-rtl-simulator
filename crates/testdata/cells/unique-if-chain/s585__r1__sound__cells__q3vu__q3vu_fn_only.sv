package pk;
  function automatic int pf(input logic x, input logic z);
    pf = 0;
    unique if (x) pf = 1; else if (z) pf = 2;
    priority if (x) pf = 1; else if (z) pf = 2; else if (x & z) pf = 3;
    unique if (x) pf = 1; else unique0 if (z) pf = 2; else if (x) pf = 3;
    unique if (x) pf = 1; else assert (z) else if (x) pf = 2;
    unique if (x) pf = 1;
  endfunction
endpackage
class C;
  int r;
  function new(input logic x, input logic z);
    unique if (x) r = 1; else if (z) r = 2;
  endfunction
  function int f(input logic x, input logic z);
    unique if (x) f = 1; else if (z) f = 2; else if (x) f = 3;
    unique if (x) f = 1;
  endfunction
  function void v(input logic x, input logic z);
    unique if (x) r = 1; else if (z) r = 2;
    begin unique if (x) r = 1; else (* a *) if (z) r = 2; end
  endfunction
endclass
interface I;
  function int fi(input logic x, input logic z);
    fi = 0; unique if (x) fi = 1; else if (z) fi = 2;
  endfunction
endinterface
module top;
  logic a = 0, b = 0;
  int q;
  I i();
  function int fn(input logic x, input logic z);
    fn = 0;
    unique if (x) fn = 1; else if (z) fn = 2;
    for (int k = 0; k < 2; k++) unique if (x) fn = 1; else if (z) fn = 2;
    case (x) 1'b1: unique if (z) fn = 1; else if (x) fn = 2; default: ; endcase
  endfunction
  localparam int P = fn(1'b0, 1'b1);
  C obj;
  initial begin
    obj = new(a, b);
    q = fn(a, b) + pk::pf(a, b) + obj.f(a, b) + i.fi(a, b) + P;
    obj.v(a, b);
    $display("q=%0d", q);
    #1 $finish;
  end
endmodule
