module top;
  function automatic int fc(input int a); case (a) 2: fc = 7; default: fc = 1; endcase endfunction
  if (1) begin : gb
    case (S)
      8'd99: begin : g wire [7:0] r = {fc(2){1'b1}}; initial #1 $display("@k r=%h", r); end
      default: begin : g initial #1 $display("@def"); end
    endcase
    localparam logic [7:0] S = 8'd99;
  end
  initial #5 $finish;
endmodule
