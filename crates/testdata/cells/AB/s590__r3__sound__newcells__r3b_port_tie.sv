module child(input logic a, output logic o);
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  assign o = f(a);
endmodule
module top;
  logic a, b;
  wire po, d, m, v;
  child u(.a(a), .o(po));
  assign #1 d = po;
  assign m = po;
  assign m = b;
  assign v = (po === 1'bz);
  always @(posedge v) $display("PV t=%0t", $time);
  always @(m) $display("M t=%0t m=%b", $time, m);
  initial begin a = 1'b0; b = 1'b0; end
  initial #1 $display("t1 po=%b d=%b m=%b v=%b", po, d, m, v);
  initial #10 $finish;
endmodule
