module top;
  localparam real Q = 2.5;
  if (1) begin : gb
    localparam P = Q;
    initial #1 $display("@P=%0d", P);
    localparam integer Q = 7;
  end
  initial #5 $finish;
endmodule
