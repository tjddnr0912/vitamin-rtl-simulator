package p;
  localparam logic signed [7:0] AS [2] = '{-8'sd4, 8'sd3};
endpackage
module t;
  localparam L3 = p::AS[0] + {4{1'b0}};
  localparam K3 = (p::AS[0] < 8'sd0);
  initial $display("R: L3=%0d B3=%0d K3=%0d", L3, $bits(L3), K3);
  initial #10 $finish;
endmodule
