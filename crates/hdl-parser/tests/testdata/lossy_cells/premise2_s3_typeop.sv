package pk; localparam int W = 70; typedef logic [W-1:0] w_t; endpackage
module t; import pk::*; logic [69:0] a; pk::w_t pv = 'z; w_t pv2;
  initial begin pv2 = 'x; #1 $display("A pv=%h pv2=%h", pv, pv2); end
  initial #100 $finish;
endmodule
