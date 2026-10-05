localparam K = 4;
module top;
  case (K)
    4: begin : g initial #1 $display("@four"); end
    default: begin : g initial #1 $display("@def"); end
  endcase
  localparam K = 8;
  initial #5 $finish;
endmodule
