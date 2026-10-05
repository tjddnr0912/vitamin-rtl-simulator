module top;
  localparam integer K = 1;
  if (1) begin : g
    if (K == 2) begin : x case (1) 1: begin : f initial #1 $display("@a"); end endcase end
    else begin : x case (1) 1: begin : f initial #1 $display("@b"); end endcase end
    localparam integer K = 2;
  end
  initial #5 $finish;
endmodule
