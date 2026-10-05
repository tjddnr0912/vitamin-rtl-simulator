`define B (4'bx100 ==? 4'bx100)
typedef logic [`B:0] t_t;
interface ifc; logic [`B:0] s; modport mp(input s); endinterface
class C; logic [`B:0] m; endclass
module chld(ifc.mp p); initial #1 $display("@modport %0d", $bits(p.s)); endmodule
module cp(input logic [`B:0] pp); initial #1 $display("@port %0d", $bits(pp)); endmodule
module top;
  t_t v1;
  ifc i0();
  chld c0(.p(i0));
  logic [`B:0] pc;
  cp c1(.pp(pc));
  localparam logic [`B:0] LP = '1;
  function automatic logic [`B:0] fr(); return '1; endfunction
  logic up [`B:0];
  logic [7:0] vv = 8'hFF;
  wire [7:0] ps = vv[0 +: `B];
  wire [7:0] rp = {`B{1'b1}};
  logic [`B*3:0] ar;
  C c;
  initial begin
    #1;
    c = new;
    $display("@typedef %0d", $bits(v1));
    $display("@ifsig %0d", $bits(i0.s));
    $display("@lp %0d", $bits(LP));
    $display("@fr %0d", $bits(fr()));
    $display("@unpk %0d", $size(up));
    $display("@psel %0d", ps);
    $display("@rep %0d", rp);
    $display("@arith %0d", $bits(ar));
    $display("@class %0d", $bits(c.m));
  end
endmodule
