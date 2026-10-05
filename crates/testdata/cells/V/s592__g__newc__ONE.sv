module top;
  localparam integer K = 1;
  if (1) begin : g
    if (K == 2) begin : x initial #1 $display("@then"); end
    else begin : x wire [3:0] w; end
    localparam integer K = 2;
  end
  initial #5 $finish;
endmodule
