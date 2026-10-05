module top;
  if (1) begin : gb
    localparam K = 4'sb1000 >>> 1;
    if (K < 0) begin : g wire [7:0] w = 8'd200; initial #1 $display("@neg %0d bits=%0d", w, $bits(w)); end
    else begin : g wire [3:0] w = 4'd9; initial #1 $display("@pos %0d bits=%0d", w, $bits(w)); end
  end
  initial #5 $finish;
endmodule
