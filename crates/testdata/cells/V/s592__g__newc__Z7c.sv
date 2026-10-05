module top;
  localparam S = 1;
  if (1) begin : gb
    if (S == 1) begin : g initial #1 $display("@then"); end
    else begin : g initial #1 $display("@else"); end
    localparam real S = 1.5;
  end
  initial #5 $finish;
endmodule
