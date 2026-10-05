module top;
  localparam [63:0] S = 64'h0000_0100_0000_0000;
  if (1) begin : g
    if (S[41:40] == 1) begin : a initial $display("@then"); end
    else begin : b initial $display("@else"); end
    localparam integer S = 3;
  end
  initial #10 $finish;
endmodule
