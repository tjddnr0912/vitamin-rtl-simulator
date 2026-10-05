module top;
  localparam logic [7:0] K = 8'd1;
  if (1) begin : gb
    if (K == 99) begin : g wire [7:0] w = 8'd200; initial #1 $display("@then %0d bits=%0d", w, $bits(w)); end
    else begin : g wire [3:0] w = 4'd9; initial #1 $display("@else %0d bits=%0d", w, $bits(w)); end
    localparam logic [7:0] K = 8'd99;
  end
  initial #5 $finish;
endmodule
