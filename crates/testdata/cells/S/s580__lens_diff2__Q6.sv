module t;
  logic [7:0] v8 = 8'hA5;
`ifdef REPX
  initial #1 $display("REPX %b", {((4'bx100 ==? 4'b1?00) + 1){1'b1}});
`endif
`ifdef REPE
  initial #1 $display("REPE %b", {((4'bx100 == 4'b1100) + 1){1'b1}});
`endif
`ifdef PSEL
  initial #1 $display("PSEL %b", v8[(4'bx100 ==? 4'b1?00)*3+1:0]);
`endif
`ifdef PSEE
  initial #1 $display("PSEE %b", v8[(4'bx100 == 4'b1100)*3+1:0]);
`endif
`ifdef XB
  case (1'b1) (4'bx100 ==? 4'b1?00): begin : gb initial #1 $display("XB x"); end 1'b1: begin : gb1 initial #1 $display("XB one"); end endcase
`endif
`ifdef XE
  case (1'b1) (4'bx100 == 4'b1100): begin : ge initial #1 $display("XE x"); end 1'b1: begin : ge1 initial #1 $display("XE one"); end endcase
`endif
`ifdef XI
  case (1'b1) (4'bx100 inside {4'b1?00}): begin : gi initial #1 $display("XI x"); end 1'b1: begin : gi1 initial #1 $display("XI one"); end endcase
`endif
`ifdef STR
  if ("a" ==? 8'b0110_000x) begin : s0 initial #1 $display("G0 1"); end else begin : s0e initial #1 $display("G0 0"); end
  if ("a" ==? 8'b0110_001x) begin : s1 initial #1 $display("G1 1"); end else begin : s1e initial #1 $display("G1 0"); end
 `ifndef IV
  if ("a" inside {8'b0110_000x}) begin : s2 initial #1 $display("G2 1"); end else begin : s2e initial #1 $display("G2 0"); end
 `else
  if ("a" ==? 8'b0110_000x) begin : s2 initial #1 $display("G2 1"); end else begin : s2e initial #1 $display("G2 0"); end
 `endif
`endif
  initial #3 $finish;
endmodule
