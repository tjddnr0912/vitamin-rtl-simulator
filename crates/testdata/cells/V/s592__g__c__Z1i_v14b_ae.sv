module top;
  function automatic logic [3:0] fx(input int a); logic [3:0] t; fx = t; endfunction
  if (1) begin : gb
    case (8'd99)
      8'd1: begin : g wire [7:0] w = 8'd1; initial #1 $display("@one %0d", w); end
      K: begin : g2 localparam logic [3:0] P = fx(1); initial #1 $display("@k P=%b", P); end
    endcase
    localparam logic [7:0] K = 8'd99;
  end
  initial #5 $finish;
endmodule
