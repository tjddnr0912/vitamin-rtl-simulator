module mp; logic [1:0] v; logic [1:0] src = 2'd1; always_comb v = src; endmodule
module mm; logic [1:0] v; always_comb v = top.u0.v; endmodule
module mc; logic [1:0] y; always_comb begin unique case (top.u1.v) 2'd1: y = 2'd1; 2'd2: y = 2'd2; endcase end endmodule
module top;
  mp u0();
  mm u1();
  mc u2();
  initial #1 $display("t=%0t y=%0d", $time, u2.y);
  initial #5 $finish;
endmodule
