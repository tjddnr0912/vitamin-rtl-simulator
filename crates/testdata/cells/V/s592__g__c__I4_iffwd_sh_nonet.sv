module top;
  localparam logic [7:0] K = 8'd1;
  if (1) begin : gb
    if (K == 99) begin : g initial #1 $display("@then"); end
    else begin : g initial #1 $display("@else"); end
    localparam logic [7:0] K = 8'd99;
  end
  initial #5 $finish;
endmodule
