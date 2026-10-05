interface ifc ();
  localparam integer K = 1;
  if (1) begin : g
    if (K == 2) begin : a initial $display("@inner"); end
    else begin : b initial $display("@outer"); end
    localparam integer K = 2;
  end
endinterface
module top;
  ifc i();
  initial #10 $finish;
endmodule
