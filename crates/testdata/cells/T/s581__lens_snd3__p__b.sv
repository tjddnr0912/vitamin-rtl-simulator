module t;
  localparam int N = 2;
`ifdef B1
  logic [("AB" ==? 16'h4?42) + 2 : 0] v;
  initial $display("ARM=%0d", $bits(v));
`elsif B2
  logic [({N{2'b10}} inside {4'b1?10}) + 2 : 0] v;
  initial $display("ARM=%0d", $bits(v));
`elsif B3
  logic [({2{2'b10}} ==? 4'b1?10) + 2 : 0] v;
  initial $display("ARM=%0d", $bits(v));
`elsif G1L
  case (1'b1)
    ({2{2'b10}} ==? 4'b1?10): begin : a initial $display("ARM=a"); end
    default: begin : d initial $display("ARM=def"); end
  endcase
`endif
  initial #5 $finish;
endmodule
