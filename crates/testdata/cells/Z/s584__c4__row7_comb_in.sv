module top;
  logic [7:0] r, d;
  child u1(.d(d));
  initial begin d = 8'h0E; r = u1.s; $display("r=%h", r); #0 $display("r#0=%h", u1.s); end
  initial #10 $finish;
endmodule
module child(input logic [7:0] d);
  logic [7:0] s;
  always_comb s = d ^ 8'hE0;
endmodule
