module top;
  logic [7:0] r;
  child u1();
  initial begin r = u1.s; $display("r=%h", r); #0 $display("r#0=%h", u1.s); end
  initial #10 $finish;
endmodule
module child;
  logic [7:0] s;
  initial s = 8'hEE;
endmodule
