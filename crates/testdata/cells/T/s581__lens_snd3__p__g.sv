module t;
  localparam int N = 2;
  localparam int W = 8;
  localparam [7:0] P = 8'hA5;
`ifdef G1
  case (1'b1)
    ({N{2'b10}} ==? 4'b1?10): begin : a initial $display("ARM=a"); end
    default: begin : d initial $display("ARM=def"); end
  endcase
`elsif G2
  case ({N{2'b10}} ==? 4'b1?10)
    1'b1: begin : a initial $display("ARM=a"); end
    default: begin : d initial $display("ARM=def"); end
  endcase
`elsif G3
  case (1'b1)
    ({N{2'b10}} !=? 4'b1?10): begin : a initial $display("ARM=a"); end
    default: begin : d initial $display("ARM=def"); end
  endcase
`elsif R1
  localparam LR = ((P & {W{1'b1}}) ==? 8'b1?10_0101);
  initial $display("ARM=%0d", LR);
`elsif R2
  logic [({N{2'b10}} ==? 4'b1?10) + 2 : 0] v;
  initial $display("ARM=%0d", $bits(v));
`elsif R5
  localparam [7:0] SC = ({N{2'b10}} ==? 4'b1?10) ? 8'd5 : 8'd9;
  initial $display("ARM=%0d", SC);
`elsif R6
  localparam [7:0] RC = {(({N{2'b10}} ==? 4'b1?10) + 1){1'b1}};
  initial $display("ARM=%0d", RC);
`elsif R7
  if (({N{2'b10}} ==? 4'b1?10) && 1'b1) begin : gt initial $display("ARM=then"); end
  else begin : ge initial $display("ARM=else"); end
`endif
  initial #5 $finish;
endmodule
