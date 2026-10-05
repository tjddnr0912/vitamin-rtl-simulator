module top;
  function automatic int fc(input int a); case (a) 2: fc = 7; default: fc = 1; endcase endfunction
  if (1) begin : gb
    if (K == 99) begin : g wire [7:0] r = {fc(2){1'b1}}; initial #1 $display("@then r=%h", r); end
    else begin : g initial #1 $display("@else"); end
    localparam logic [7:0] K = 8'd99;
  end
  initial #5 $finish;
endmodule
