`timescale 1ns/1ns
module t;
  localparam bit S = 1'b1;
  case (S)
`ifdef XV
    (4'bx100 ==? 4'b1?00): begin : g1 initial $display("G1 item"); end
`elsif XI
    (4'bx100 inside {4'b1?00}): begin : g1 initial $display("G1 item"); end
`else
    (4'b1100 ==? 4'b1?00): begin : g1 initial $display("G1 item"); end
`endif
    default: begin : g1d initial $display("G1 default"); end
  endcase
  initial #1 $finish;
endmodule
