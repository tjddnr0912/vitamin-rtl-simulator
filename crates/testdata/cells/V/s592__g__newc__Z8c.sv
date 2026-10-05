module top;
  localparam S = 1;
  if (1) begin : gb
    if (S == 1) begin : g initial #1 $display("@then"); end
    else begin : g initial #1 $display("@else"); end
    localparam logic [71:0] S = 72'hFF_0000_0000_0000_0001;
  end
  initial #5 $finish;
endmodule
